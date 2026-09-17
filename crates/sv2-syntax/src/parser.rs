// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A parser for the package declaration, written from `SysML` 8.2.2.5.1.
//!
//! The productions it implements, as the specification states them:
//!
//! ```text
//! RootNamespace      = PackageBodyElement*
//! PackageBodyElement = PackageMember | ElementFilterMember | AliasMember | Import
//! PackageMember      = MemberPrefix ( DefinitionElement | UsageElement )
//! MemberPrefix       = ( visibility = VisibilityIndicator )?
//! Package            = ( ownedRelationship += PrefixMetadataMember )*
//!                      PackageDeclaration PackageBody
//! PackageDeclaration = 'package' Identification
//! PackageBody        = ';' | '{' PackageBodyElement* '}'
//! Identification     = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
//! Import             = VisibilityIndicator 'import' 'all'?
//!                      ImportDeclaration RelationshipBody
//! AliasMember        = MemberPrefix 'alias' ( '<' NAME '>' )? NAME?
//!                      'for' [QualifiedName] RelationshipBody
//! RelationshipBody   = ';' | '{' OwnedAnnotation* '}'
//! PartDefinition     = OccurrenceDefinitionPrefix 'part' 'def' Definition
//! ```
//!
//! Three of the four `PackageBodyElement` alternatives are handled: `PackageMember`,
//! `Import` and `AliasMember`. `ElementFilterMember` waits on `OwnedExpression`.
//! `Package` and `PartDefinition` are the two `DefinitionElement`s of thirty, and
//! `Comment`, `Documentation` and `TextualRepresentation` the three `AnnotatingElement`s
//! of four that a `RelationshipBody` may own. Everything else is unimplemented and
//! reports as such in the coverage report, which is the honest state of a parser this
//! young.
//!
//! Every token the lexer produced ends up in the tree, in source order. Text that no
//! production accepts becomes an `Error` node that still carries its bytes, so the
//! round-trip holds for malformed input.

use rowan::{GreenNode, GreenNodeBuilder, Language as _};

use crate::generated::kinds::{KEYWORDS, SyntaxKind};
use crate::language::{Sv2Language, SyntaxNode};
use crate::lexer::{Token, is_trivia, is_unterminated_comment, tokenize};

/// The result of parsing: a tree, plus what went wrong.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<String>,
}

impl Parse {
    /// The root of the tree.
    #[must_use]
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// The source text, reconstructed from the tree.
    ///
    /// Equal to the input for every input. This is the losslessness invariant.
    #[must_use]
    pub fn text(&self) -> String {
        self.syntax().text().to_string()
    }

    /// What the parser could not make sense of, in source order.
    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

/// The kind the pinned token set gives `text`, or `None` if it names no keyword.
///
/// The keyword table is generated from the pinned token set, so asking it is what
/// keeps this parser tied to the pin rather than to a constant written out here.
///
/// The lookup returning `None` does not fail loudly — the caller falls back to
/// `BasicName`, because a parser that panics because a token set moved is worse than
/// one that mis-tags a node. What makes the fallback safe to have is
/// `every_keyword_this_parser_names_is_in_the_pinned_token_set` below: a keyword
/// leaving the token set fails the gate there, not silently at run time.
fn keyword(text: &str) -> Option<SyntaxKind> {
    KEYWORDS
        .iter()
        .find(|(k, _)| *k == text)
        .map(|(_, kind)| *kind)
}

/// The three `VisibilityIndicator` keywords, in the order the specification
/// writes them (`SysML` 8.2.2.5.1). Looked up in the pinned token set like every
/// other keyword; this is only the list of which ones the production names.
const VISIBILITY: [&str; 3] = ["public", "private", "protected"];

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<String>,
    /// Whether a `REGULAR_COMMENT` is a token here rather than trivia.
    ///
    /// `KerML` 8.2.2.2 makes `/* ... */` a token, and `Comment`, `Documentation` and
    /// `TextualRepresentation` take it as their body (`SysML` 8.2.2.4.2, 8.2.2.4.3).
    /// Inside a braced `RelationshipBody` every regular comment is therefore either an
    /// annotation's body or an error, never text to skip past. Elsewhere this parser
    /// still attaches it as trivia, because the `AnnotatingMember` that would own it
    /// as an element in a package or definition body is not implemented.
    comments_significant: bool,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            tokens: tokenize(source),
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            comments_significant: false,
        }
    }

    fn text_of(&self, token: Token) -> &'a str {
        token.text(self.source).unwrap_or("")
    }

    // -- looking ahead ----------------------------------------------------------

    /// The next non-trivia token, without consuming anything.
    fn peek(&self) -> Option<Token> {
        self.peek_nth(0)
    }

    /// The `n`th non-trivia token from here, without consuming anything.
    ///
    /// `ImportDeclaration` needs two: `A::B` and `A::*` differ only after the `::`,
    /// and the `::` belongs to the `QualifiedName` in one and to the import in the
    /// other.
    fn peek_nth(&self, n: usize) -> Option<Token> {
        self.tokens
            .get(self.pos..)?
            .iter()
            .filter(|token| !self.skippable(token.kind))
            .nth(n)
            .copied()
    }

    /// Whether `kind` is trivia in the current context, and so skipped by lookahead.
    fn skippable(&self, kind: SyntaxKind) -> bool {
        is_trivia(kind) && !(self.comments_significant && kind == SyntaxKind::RegularComment)
    }

    /// Run `production` with `REGULAR_COMMENT` treated as a token, then restore.
    fn with_significant_comments(&mut self, production: impl FnOnce(&mut Self)) {
        let outer = self.comments_significant;
        self.comments_significant = true;
        production(self);
        self.comments_significant = outer;
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// Whether the next meaningful token is a NAME.
    ///
    /// NAME is `BASIC_NAME` | `UNRESTRICTED_NAME` (`KerML` 8.2.2.3).
    ///
    /// A reserved word is excluded. `KerML` 8.2.2.6: "a reserved keyword is a token
    /// that has the lexical structure of a basic name but cannot actually be used as a
    /// basic name". The lexer cannot make that distinction, because `package` and
    /// `Vehicle` are the same token shape; the pinned keyword table is what separates
    /// them, and asking it here is what keeps `package package;` from declaring a
    /// package named `package`.
    fn at_name(&self) -> bool {
        self.peek().is_some_and(|token| self.is_name(token))
    }

    /// Whether `token` is a NAME, asked of any token rather than only the next one.
    fn is_name(&self, token: Token) -> bool {
        match token.kind {
            SyntaxKind::UnrestrictedName => true,
            SyntaxKind::BasicName => keyword(self.text_of(token)).is_none(),
            _ => false,
        }
    }

    /// Whether a `VisibilityIndicator` starts here (`SysML` 8.2.2.5.1).
    fn at_visibility(&self) -> bool {
        VISIBILITY.iter().any(|word| self.at_keyword(word))
    }

    /// Whether the next meaningful token is this keyword.
    fn at_keyword(&self, text: &str) -> bool {
        self.nth_is_keyword(0, text)
    }

    /// Whether the `n`th meaningful token from here is this keyword.
    fn nth_is_keyword(&self, n: usize, text: &str) -> bool {
        self.peek_nth(n)
            .is_some_and(|token| token.kind == SyntaxKind::BasicName && self.text_of(token) == text)
    }

    /// Whether the keyword deciding which `PackageBodyElement` this is, is `text`.
    ///
    /// Looks past an optional `VisibilityIndicator`. `MemberPrefix`'s visibility is
    /// optional and `Import`'s is required, so the indicator never decides on its
    /// own — the keyword after it does.
    fn at_element_keyword(&self, text: &str) -> bool {
        self.nth_is_keyword(usize::from(self.at_visibility()), text)
    }

    /// Whether an `Import` starts here rather than a `PackageMember`.
    ///
    /// Both may open with a `VisibilityIndicator`, so the indicator alone does not
    /// say which: `Import`'s visibility is required and `MemberPrefix`'s is
    /// optional. The keyword after it is what separates them. A bare `import` with
    /// no visibility is neither, and falls through to recovery — which is the rule
    /// tests/rejection/import-without-visibility.sysml holds.
    fn at_import(&self) -> bool {
        self.at_visibility() && self.nth_is_keyword(1, "import")
    }

    /// Whether a `PartDefinition` starts at the `n`th meaningful token.
    ///
    /// `PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition`
    /// (`SysML` 8.2.2.11), with `OccurrenceDefinitionPrefix = BasicDefinitionPrefix?
    /// ( 'individual' EmptyMultiplicityMember )? DefinitionExtensionKeyword*`
    /// (8.2.2.9.1). The prefix keywords are optional and also open usages
    /// (`abstract part x;` is a `PartUsage`), so only `part` followed by `def` decides.
    /// A `DefinitionExtensionKeyword` (`#` prefix metadata) is not looked past: it is
    /// unimplemented, and leaving it to the enclosing body's recovery reports it.
    fn at_part_definition(&self, n: usize) -> bool {
        let mut n = n;
        if self.nth_is_keyword(n, "abstract") || self.nth_is_keyword(n, "variation") {
            n += 1;
        }
        if self.nth_is_keyword(n, "individual") {
            n += 1;
        }
        self.nth_is_keyword(n, "part") && self.nth_is_keyword(n + 1, "def")
    }

    /// Whether an implemented `DefinitionElement` starts at the `n`th meaningful token.
    fn at_definition_element(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "package") || self.at_part_definition(n)
    }

    /// Whether an implemented `AnnotatingElement` starts here (`SysML` 8.2.2.4.1).
    ///
    /// `Comment` may open with `comment`, `locale` or its bare `REGULAR_COMMENT` body;
    /// `Documentation` with `doc`; `TextualRepresentation` with `rep` or `language`.
    /// `MetadataUsage` (`@`, `metadata`, or a `#` extension keyword) is not
    /// implemented, so it is not recognised and its tokens are reported.
    fn at_annotating_element(&self) -> bool {
        self.at(SyntaxKind::RegularComment)
            || ["comment", "locale", "doc", "rep", "language"]
                .iter()
                .any(|word| self.at_keyword(word))
    }

    fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    // -- building the tree ------------------------------------------------------

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(Sv2Language::kind_to_raw(kind));
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Attach pending trivia to the tree. Never skipped — losslessness depends on it.
    fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos).copied() {
            if !self.skippable(token.kind) {
                break;
            }
            self.push(token, token.kind);
        }
    }

    /// Every token enters the tree here exactly once, which is why the
    /// unterminated-comment diagnostic is raised here: a regular comment may arrive
    /// as trivia or as an annotation's body, and both must report it.
    fn push(&mut self, token: Token, kind: SyntaxKind) {
        if is_unterminated_comment(token.kind, self.text_of(token)) {
            self.errors
                .push("comment is never closed: expected `*/`".to_owned());
        }
        self.builder
            .token(Sv2Language::kind_to_raw(kind), self.text_of(token));
        self.pos += 1;
    }

    /// Consume the next non-trivia token, with the trivia before it.
    fn bump(&mut self) {
        if let Some(token) = self.peek() {
            self.bump_as(token.kind);
        }
    }

    /// Consume the next non-trivia token, tagged as `kind` rather than as it lexed.
    ///
    /// Keywords reach the parser as `BasicName`; the production that recognises one
    /// says so here, and the tree records the keyword the token set names.
    fn bump_as(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos).copied() {
            self.push(token, kind);
        }
    }

    // -- diagnostics and recovery -----------------------------------------------

    /// Consume the next token as `kind`, or record an error without consuming.
    fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if self.at(kind) {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    /// Consume `text` as a keyword, or record an error without consuming.
    fn expect_keyword(&mut self, text: &str) {
        if self.at_keyword(text) {
            self.bump_as(keyword(text).unwrap_or(SyntaxKind::BasicName));
        } else {
            self.error_expected(&format!("`{text}`"));
        }
    }

    /// Consume the next token as a NAME, or record an error without consuming.
    fn expect_name(&mut self, what: &str) {
        if self.at_name() {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    fn error_expected(&mut self, what: &str) {
        let found = self
            .peek()
            .map_or("end of file", |token| self.text_of(token));
        self.errors
            .push(format!("expected {what}, found `{found}`"));
    }

    /// One token nothing accepts, wrapped so its bytes survive in the tree.
    fn error_token(&mut self) {
        self.eat_trivia();
        let Some(token) = self.tokens.get(self.pos).copied() else {
            return;
        };
        self.errors
            .push(format!("unexpected `{}`", self.text_of(token)));
        self.start_node(SyntaxKind::Error);
        self.push(token, token.kind);
        self.finish_node();
    }

    // -- productions ------------------------------------------------------------

    // production: RootNamespace
    fn root_namespace(mut self) -> (GreenNode, Vec<String>) {
        self.start_node(SyntaxKind::RootNamespace);
        self.body_elements(None, SyntaxKind::PackageMember);
        // Trailing trivia belongs to the tree as much as anything else.
        self.eat_trivia();
        self.finish_node();
        (self.builder.finish(), self.errors)
    }

    /// `PackageBodyElement*` or `DefinitionBodyItem*`, up to `until` or end of input.
    ///
    /// `PackageBodyElement = PackageMember | ElementFilterMember | AliasMember |
    /// Import` (`SysML` 8.2.2.5.1). All but `ElementFilterMember` are implemented,
    /// and that one waits on `OwnedExpression`.
    ///
    /// `DefinitionBodyItem = DefinitionMember | VariantUsageMember |
    /// NonOccurrenceUsageMember | SourceSuccessionMember? OccurrenceUsageMember |
    /// AliasMember | Import` (`SysML` 8.2.2.6.1). `DefinitionMember`, `AliasMember`
    /// and `Import` are implemented; the usage members are not. `member` is the
    /// membership node a nested `DefinitionElement` is owned through, which is the
    /// one thing that differs between the two bodies.
    ///
    /// Anything else is recovered over one token at a time rather than abandoning
    /// the enclosing body: an editor reparses invalid text constantly, and a body
    /// that vanishes on one bad token blanks the diagram on every keystroke.
    fn body_elements(&mut self, until: Option<SyntaxKind>, member: SyntaxKind) {
        while !self.at_end() && !until.is_some_and(|kind| self.at(kind)) {
            if self.at_import() {
                self.import();
            } else if self.at_element_keyword("alias") {
                self.alias_member();
            } else if self.at_definition_element(usize::from(self.at_visibility())) {
                self.membership(member);
            } else {
                self.error_token();
            }
        }
    }

    // production: AliasMember
    //
    // AliasMember : Membership =
    //     MemberPrefix
    //     'alias' ( '<' memberShortName = NAME '>' )?
    //     ( memberName = NAME )?
    //     'for' memberElement = [QualifiedName]
    //     RelationshipBody
    //
    // `Membership`, not `OwningMembership` — the distinction the production exists
    // for. `KerML` 8.3.2.4.3: a Membership that does not own its memberElement makes
    // its memberNames "effectively aliases within the membershipOwningNamespace for
    // an Element with a separate OwningMembership". That is why the target is a
    // `[QualifiedName]` reference to an element declared elsewhere, and not a nested
    // element the way `PackageMember`'s is.
    //
    // Both name slots are optional and both are 0..1 on the metaclass, so
    // `alias for X;` is well formed as far as the syntax goes. The short-name slot is
    // not exercised by the pinned corpus — the only `alias <` in 311 files is inside
    // a comment — so its test is constructed from the production rather than found.
    fn alias_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AliasMember);
        self.member_prefix();
        self.bump_as(keyword("alias").unwrap_or(SyntaxKind::BasicName));
        if self.at(SyntaxKind::Lt) {
            self.bump();
            self.expect_name("a short name");
            self.expect(SyntaxKind::Gt, "`>`");
        }
        if self.at_name() {
            self.bump();
        }
        self.expect_keyword("for");
        self.qualified_name();
        self.relationship_body();
        self.finish_node();
    }

    // production: PackageMember
    //
    // PackageMember : OwningMembership =
    //     MemberPrefix ( ownedRelatedElement += DefinitionElement
    //                  | ownedRelatedElement = UsageElement )
    //
    // production: DefinitionMember
    //
    // DefinitionMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += DefinitionElement      (SysML 8.2.2.6.1)
    //
    // The two differ only in PackageMember's UsageElement alternative, which is not
    // implemented, so one method builds both under the node the caller names.
    //
    // `DefinitionElement` and `UsageElement` get no node of their own. They are
    // alternations over element productions, and the element that matched already
    // says which alternative was taken, so a node here would add a level carrying
    // nothing. Neither is marked for coverage: `Package` and `PartDefinition` are
    // the only two of `DefinitionElement`'s 30 alternatives implemented, and
    // `UsageElement` none.
    fn membership(&mut self, member: SyntaxKind) {
        self.eat_trivia();
        self.start_node(member);
        self.member_prefix();
        if self.at_keyword("package") {
            self.package();
        } else if self.at_part_definition(0) {
            self.part_definition();
        } else {
            self.error_expected("a package or a part definition");
        }
        self.finish_node();
    }

    // production: PartDefinition
    //
    // PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition
    //                                                            (SysML 8.2.2.11)
    //
    // The Pilot factors 'part' 'def' into PartDefKeyword; deviations.json records
    // that as xtext_only/follow_spec, so the literals are matched here directly.
    fn part_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PartDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("part");
        self.expect_keyword("def");
        self.definition();
        self.finish_node();
    }

    // production: OccurrenceDefinitionPrefix
    //
    // OccurrenceDefinitionPrefix : OccurrenceDefinition =
    //     BasicDefinitionPrefix?
    //     ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
    //     DefinitionExtensionKeyword*                            (SysML 8.2.2.9.1)
    //
    // DefinitionExtensionKeyword (`#` prefix metadata, a PrefixMetadataMember) is not
    // implemented. at_part_definition does not look past a `#`, so a definition that
    // carries one never reaches here: the enclosing body reports the `#` and its
    // name token by token, then parses the definition after them.
    //
    // The node is built even when every slot is empty, as MemberPrefix's is.
    fn occurrence_definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OccurrenceDefinitionPrefix);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
        if self.at_keyword("individual") {
            self.bump_as(keyword("individual").unwrap_or(SyntaxKind::BasicName));
            self.empty_multiplicity_member();
        }
        self.finish_node();
    }

    // production: BasicDefinitionPrefix
    //
    // BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?= 'variation'
    //                                                            (SysML 8.2.2.6.1)
    fn basic_definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicDefinitionPrefix);
        match ["abstract", "variation"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`abstract` or `variation`"),
        }
        self.finish_node();
    }

    // production: EmptyMultiplicityMember
    //
    // EmptyMultiplicityMember : OwningMembership =
    //     ownedRelatedElement += EmptyMultiplicity               (SysML 8.2.2.9.1)
    //
    // production: EmptyMultiplicity
    //
    // EmptyMultiplicity : Multiplicity = { }
    //
    // No tokens, but a real element: `individual` gives the definition an owned
    // Multiplicity. The nodes are empty rather than omitted, so the tree carries the
    // element the abstract syntax says is there.
    fn empty_multiplicity_member(&mut self) {
        self.start_node(SyntaxKind::EmptyMultiplicityMember);
        self.start_node(SyntaxKind::EmptyMultiplicity);
        self.finish_node();
        self.finish_node();
    }

    // production: Definition
    //
    // Definition = DefinitionDeclaration DefinitionBody          (SysML 8.2.2.6.1)
    fn definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Definition);
        self.definition_declaration();
        self.definition_body();
        self.finish_node();
    }

    // production: DefinitionDeclaration
    //
    // DefinitionDeclaration : Definition = Identification SubclassificationPart?
    //                                                            (SysML 8.2.2.6.1)
    fn definition_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionDeclaration);
        self.identification();
        if self.at(SyntaxKind::ColonGt) || self.at_keyword("specializes") {
            self.subclassification_part();
        }
        self.finish_node();
    }

    // production: SubclassificationPart
    //
    // SubclassificationPart : Classifier =
    //     SPECIALIZES ownedRelationship += OwnedSubclassification
    //     ( ',' ownedRelationship += OwnedSubclassification )*   (SysML 8.2.2.6.5)
    //
    // SPECIALIZES = ':>' | 'specializes'                         (KerML 8.2.2.7)
    fn subclassification_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SubclassificationPart);
        if self.at(SyntaxKind::ColonGt) {
            self.bump();
        } else {
            self.expect_keyword("specializes");
        }
        self.owned_subclassification();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_subclassification();
        }
        self.finish_node();
    }

    // production: OwnedSubclassification
    //
    // OwnedSubclassification : Subclassification = superClassifier = [QualifiedName]
    //                                                            (SysML 8.2.2.6.5)
    fn owned_subclassification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedSubclassification);
        self.qualified_name();
        self.finish_node();
    }

    // production: DefinitionBody
    //
    // DefinitionBody : Type = ';' | '{' DefinitionBodyItem* '}'  (SysML 8.2.2.6.1)
    //
    // DefinitionBodyItem is not marked: three of its six alternatives are
    // implemented (see body_elements).
    fn definition_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.body_elements(Some(SyntaxKind::RBrace), SyntaxKind::DefinitionMember);
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.errors
                .push("expected `;` or `{` after a definition declaration".to_owned());
        }
        self.finish_node();
    }

    // production: MemberPrefix
    //
    // MemberPrefix : Membership = ( visibility = VisibilityIndicator )?
    //
    // The node is built whether or not a visibility is there. An empty one is the
    // honest shape: the slot exists in the production, and a tree that omits the
    // node when the slot is empty makes every consumer handle two shapes for one
    // construct.
    fn member_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MemberPrefix);
        if self.at_visibility() {
            self.visibility_indicator();
        }
        self.finish_node();
    }

    // production: Package
    fn package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Package);
        self.package_declaration();
        self.package_body();
        self.finish_node();
    }

    // production: PackageDeclaration
    fn package_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PackageDeclaration);
        self.bump_as(keyword("package").unwrap_or(SyntaxKind::BasicName));
        self.identification();
        self.finish_node();
    }

    // production: Identification
    fn identification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Identification);
        if self.at(SyntaxKind::Lt) {
            self.bump();
            self.expect_name("a short name");
            self.expect(SyntaxKind::Gt, "`>`");
        }
        if self.at_name() {
            self.bump();
        }
        self.finish_node();
    }

    // production: Import
    //
    // Import = visibility = VisibilityIndicator 'import' ( isImportAll ?= 'all' )?
    //          ImportDeclaration RelationshipBody
    //
    // visibility carries no `( )?`: an import states its visibility. The
    // specification BNF, the Pilot's ImportPrefix fragment, and all 741 imports in
    // the pinned corpus agree, and `MemberPrefix`'s optional visibility one clause
    // away is what makes that worth stating rather than assuming.
    fn import(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Import);
        self.visibility_indicator();
        self.bump_as(keyword("import").unwrap_or(SyntaxKind::BasicName));
        if self.at_keyword("all") {
            self.bump_as(keyword("all").unwrap_or(SyntaxKind::BasicName));
        }
        self.import_declaration();
        self.relationship_body();
        self.finish_node();
    }

    // production: VisibilityIndicator
    fn visibility_indicator(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::VisibilityIndicator);
        match VISIBILITY.iter().find(|word| self.at_keyword(word)) {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            // Unreachable from body_elements, which only enters on at_visibility.
            // Reported rather than consumed, so a future caller cannot lose a token.
            None => self.error_expected("`public`, `private` or `protected`"),
        }
        self.finish_node();
    }

    // production: ImportDeclaration
    //
    // ImportDeclaration = MembershipImport | NamespaceImport
    // MembershipImport  = [QualifiedName] ( '::' isRecursive ?= '**' )?
    // NamespaceImport   = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
    //
    // Which one it is cannot be known until after the QualifiedName, because they
    // share that prefix. The node is opened retroactively at a checkpoint rather
    // than guessed and repaired.
    //
    // NamespaceImport's second alternative, `importedNamespace = FilterPackage`, is
    // not implemented; it stays unimplemented in the coverage report.
    fn import_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ImportDeclaration);
        let inner = self.builder.checkpoint();
        self.qualified_name();
        let kind = self.import_suffix();
        self.builder
            .start_node_at(inner, Sv2Language::kind_to_raw(kind));
        self.finish_node();
        self.finish_node();
    }

    /// What follows the `QualifiedName`, and therefore which import this is.
    fn import_suffix(&mut self) -> SyntaxKind {
        if !self.at(SyntaxKind::ColonColon) {
            return SyntaxKind::MembershipImport;
        }
        self.bump();
        if self.at(SyntaxKind::Star) {
            self.bump();
            self.recursive_suffix();
            return SyntaxKind::NamespaceImport;
        }
        if self.at(SyntaxKind::StarStar) {
            self.bump();
            return SyntaxKind::MembershipImport;
        }
        // A `::` that qualified_name left behind is followed by neither, so it ends
        // the name with nothing after it.
        self.error_expected("`*` or `**` after `::`");
        SyntaxKind::MembershipImport
    }

    /// `( '::' isRecursive ?= '**' )?`, the recursive suffix a `NamespaceImport` may carry.
    fn recursive_suffix(&mut self) {
        if self.at(SyntaxKind::ColonColon) {
            self.bump();
            self.expect(SyntaxKind::StarStar, "`**`");
        }
    }

    // production: QualifiedName
    //
    // QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME   (`KerML` 8.2.3.4.1)
    //
    // A `::` is only part of the name when a NAME follows it. `A::*` ends the name at
    // `A`, and the `::` belongs to the NamespaceImport — which is why this needs two
    // tokens of lookahead rather than consuming the separator and backing out.
    fn qualified_name(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::QualifiedName);
        if self.at(SyntaxKind::Dollar) {
            self.bump();
            self.expect(SyntaxKind::ColonColon, "`::`");
        }
        self.expect_name("a name");
        while self.at(SyntaxKind::ColonColon) && self.name_follows_separator() {
            self.bump();
            self.expect_name("a name");
        }
        self.finish_node();
    }

    /// Whether the token after the next `::` is a NAME, so the `::` is the name's.
    fn name_follows_separator(&self) -> bool {
        self.peek_nth(1).is_some_and(|token| self.is_name(token))
    }

    // production: RelationshipBody
    //
    // RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
    //                                                            (SysML 8.2.2.2)
    //
    // Inside the braces a regular comment is a token (see comments_significant), so
    // `{ /* text */ }` owns a Comment rather than skipping one — the corpus's
    // `private import Definitions::* { /* ... */ }` is exactly that. Anything that is
    // not an implemented AnnotatingElement, MetadataUsage included, is recovered over
    // one token at a time and reported: accepting it silently would report an
    // annotation this parser cannot read as one it understood.
    fn relationship_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RelationshipBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.with_significant_comments(Self::owned_annotations);
        } else {
            self.errors
                .push("expected `;` or `{` after an import declaration".to_owned());
        }
        self.finish_node();
    }

    /// `OwnedAnnotation* '}'`, the rest of a braced `RelationshipBody`.
    fn owned_annotations(&mut self) {
        while !self.at_end() && !self.at(SyntaxKind::RBrace) {
            if self.at_annotating_element() {
                self.owned_annotation();
            } else {
                self.error_token();
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
    }

    // production: OwnedAnnotation
    //
    // OwnedAnnotation : Annotation = ownedRelatedElement += AnnotatingElement
    //                                                            (SysML 8.2.2.4.1)
    //
    // AnnotatingElement = Comment | Documentation | TextualRepresentation
    //                   | MetadataUsage
    //
    // The fourth alternative is MetadataUsage by deviation AnnotatingElement
    // (follow_xtext); the clause prints MetadataFeature. It is not implemented, so
    // AnnotatingElement gets no marker and no node — like DefinitionElement it is an
    // alternation whose matched element already says which alternative was taken.
    // The caller only enters on at_annotating_element, so the dispatch below never
    // sees a MetadataUsage.
    fn owned_annotation(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::OwnedAnnotation);
            if p.at_keyword("doc") {
                p.documentation();
            } else if p.at_keyword("rep") || p.at_keyword("language") {
                p.textual_representation();
            } else {
                p.comment();
            }
            p.finish_node();
        });
    }

    // production: Comment
    //
    // Comment =
    //     ( 'comment' Identification
    //       ( 'about' ownedRelationship += Annotation
    //         ( ',' ownedRelationship += Annotation )* )? )?
    //     ( 'locale' locale = STRING_VALUE )?
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.2)
    //
    // `locale` belongs after the whole optional header, not inside it.
    fn comment(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::Comment);
            if p.at_keyword("comment") {
                p.comment_header();
            }
            p.locale();
            p.expect(SyntaxKind::RegularComment, "a comment body `/* ... */`");
            p.finish_node();
        });
    }

    /// `'comment' Identification ( 'about' Annotation ( ',' Annotation )* )?`.
    fn comment_header(&mut self) {
        self.bump_as(keyword("comment").unwrap_or(SyntaxKind::BasicName));
        self.identification();
        if !self.at_keyword("about") {
            return;
        }
        self.bump_as(keyword("about").unwrap_or(SyntaxKind::BasicName));
        self.annotation();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.annotation();
        }
    }

    // production: Documentation
    //
    // Documentation =
    //     'doc' Identification ( 'locale' locale = STRING_VALUE )?
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.2)
    fn documentation(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::Documentation);
            p.bump_as(keyword("doc").unwrap_or(SyntaxKind::BasicName));
            p.identification();
            p.locale();
            p.expect(
                SyntaxKind::RegularComment,
                "a documentation body `/* ... */`",
            );
            p.finish_node();
        });
    }

    // production: TextualRepresentation
    //
    // TextualRepresentation =
    //     ( 'rep' Identification )?
    //     'language' language = STRING_VALUE
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.3)
    fn textual_representation(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::TextualRepresentation);
            if p.at_keyword("rep") {
                p.bump_as(keyword("rep").unwrap_or(SyntaxKind::BasicName));
                p.identification();
            }
            p.expect_keyword("language");
            p.expect(SyntaxKind::StringValue, "a language name string");
            p.expect(
                SyntaxKind::RegularComment,
                "a representation body `/* ... */`",
            );
            p.finish_node();
        });
    }

    /// `( 'locale' locale = STRING_VALUE )?`, shared by `Comment` and `Documentation`.
    fn locale(&mut self) {
        if self.at_keyword("locale") {
            self.bump_as(keyword("locale").unwrap_or(SyntaxKind::BasicName));
            self.expect(SyntaxKind::StringValue, "a locale string");
        }
    }

    // production: Annotation
    //
    // Annotation = annotatedElement = [QualifiedName]            (SysML 8.2.2.4.1)
    fn annotation(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Annotation);
        self.qualified_name();
        self.finish_node();
    }

    // production: PackageBody
    fn package_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PackageBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.body_elements(Some(SyntaxKind::RBrace), SyntaxKind::PackageMember);
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.errors
                .push("expected `;` or `{` after a package declaration".to_owned());
        }
        self.finish_node();
    }
}

/// Parse `SysML` v2 or `KerML` text into a lossless tree.
///
/// Never fails and never panics: text no production accepts becomes `Error` nodes
/// that keep their bytes, so `parse(s).text() == s` for every `s`.
#[must_use]
pub fn parse(source: &str) -> Parse {
    let (green, errors) = Parser::new(source).root_namespace();
    Parse { green, errors }
}

#[cfg(test)]
mod tests {
    use super::{SyntaxKind, VISIBILITY, keyword};

    /// Every keyword this parser looks up by text must exist in the pinned token set.
    ///
    /// `package_declaration` falls back to `BasicName` when the lookup misses, so a
    /// keyword leaving the token set would not fail at run time — it would quietly
    /// mis-tag the node. This is what turns that into a gate failure instead. Extend
    /// the list as productions are added.
    /// Every keyword a production looks up by text, beyond the `VISIBILITY` table
    /// the test below chains on. Extend as productions are added.
    const NAMED: &[&str] = &[
        "package",
        "import",
        "all",
        "alias",
        "for",
        "part",
        "def",
        "abstract",
        "variation",
        "individual",
        "specializes",
        "comment",
        "about",
        "locale",
        "doc",
        "rep",
        "language",
    ];

    #[test]
    fn every_keyword_this_parser_names_is_in_the_pinned_token_set() {
        // VISIBILITY is chained rather than copied: it is the list the parser itself
        // uses, so a word added there cannot drift out of this check.
        for text in NAMED.iter().chain(VISIBILITY.iter()) {
            assert!(
                keyword(text).is_some(),
                "`{text}` is not in the pinned token set; \
                 .claude/state/grammar/keywords.json moved and this parser did not"
            );
        }
    }

    #[test]
    fn a_word_that_is_not_a_keyword_has_no_kind() {
        // The negative case: the lookup must not match everything, or the fallback
        // above would never be exercised and the test would prove nothing.
        assert_eq!(keyword("Vehicle"), None);
        assert_eq!(keyword(""), None);
    }

    #[test]
    fn the_package_keyword_is_not_tagged_as_a_name() {
        // The property the snapshots pin as a side effect, stated directly.
        assert_ne!(keyword("package"), Some(SyntaxKind::BasicName));
    }
}
