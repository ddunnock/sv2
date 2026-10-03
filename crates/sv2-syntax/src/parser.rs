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
//! ElementFilterMember = MemberPrefix 'filter' OwnedExpression ';'
//! OwnedExpression    = ConditionalExpression | ConditionalBinaryOperatorExpression
//!                    | BinaryOperatorExpression | UnaryOperatorExpression
//!                    | ClassificationExpression | MetaclassificationExpression
//!                    | ExtentExpression | PrimaryExpression
//! ValuePart          = FeatureValue
//! MultiplicityPart   = OwnedMultiplicity
//!                    | OwnedMultiplicity? ( 'ordered' 'nonunique'?
//!                                         | 'nonunique' 'ordered'? )
//! ```
//!
//! `OwnedExpression` carries no precedence and cannot: `KerML` 8.2.5.8.1 note 2 states
//! that the grouping of nested `OperatorExpression`s is not expressed in the
//! productions and is given by that clause's table 6. The table is data at
//! `docs/operator-precedence.toml`, and `INFIX` below is what the parser reads. The
//! operator core reads all fifteen tiers, and the postfix layer of 8.2.5.8.2 reads
//! all six of its forms: `a.b`, `x[kg]`, `x#(1)`, `x->f()`, `x.?{ ... }` and `x.{ ... }`.
//!
//! All four `PackageBodyElement` alternatives are handled: `PackageMember`, `Import`,
//! `AliasMember` and `ElementFilterMember`, the last of which waited on the expression
//! layer and needs only a bare classification operator (`filter @Safety;`).
//! `Package` and `PartDefinition` are the two `DefinitionElement`s of thirty, and
//! `Comment`, `Documentation` and `TextualRepresentation` the three `AnnotatingElement`s
//! of four that a `RelationshipBody` may own. Everything else is unimplemented and
//! reports as such in the coverage report, which is the honest state of a parser this
//! young.
//!
//! Every token the lexer produced ends up in the tree, in source order. Text that no
//! production accepts becomes an `Error` node that still carries its bytes, so the
//! round-trip holds for malformed input.

mod action;
mod action_node;
mod annotation;
mod body;
mod classifier;
mod connection;
mod definition;
mod expression;
mod feature;
mod flow;
mod import;
mod invocation;
mod kernel;
mod literal;
mod lookahead;
mod metadata;
mod multiplicity;
mod operator;
mod primary;
mod recovery;
mod succession;
mod tree;
mod usage;

use std::cell::Cell;

use rowan::{GreenNode, GreenNodeBuilder};

use crate::diagnostic::Diagnostic;
use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::language::SyntaxNode;
use crate::lexer::{Token, is_trivia, tokenize};
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;
use crate::parser::operator::TIER_LOOSEST;
use crate::parser::usage::UsageClass;

pub use crate::parser::operator::infix_table_for_test;

/// The result of parsing: a tree, plus what went wrong.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<Diagnostic>,
    deviations: Vec<Diagnostic>,
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
    ///
    /// Each carries the range it is about, so a caller can point at it: underline the
    /// token in an editor, decorate the row in a diagram (ADR-0002), or print a line
    /// and column. A rendered sentence cannot be pointed at.
    #[must_use]
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    /// The places the text is admitted only because of a recorded deviation from the
    /// specification's BNF, in source order (ADR-0022).
    ///
    /// Each is a `PARSE-DEVIATION` diagnostic of severity `Info` whose message names its
    /// entry in `.claude/state/deviations.json`. They are NOT errors: the text parses, and
    /// [`Parse::errors`] does not contain them. A caller that needs the specification's
    /// literal reading treats them as errors — `sv2 parse --strict` does.
    ///
    /// Only EXTENSIONS are reported. A deviation that restricts the specification rejects
    /// text, and rejected text has no tree to carry a note; the register says which way
    /// each deviation runs.
    #[must_use]
    pub fn deviations(&self) -> &[Diagnostic] {
        &self.deviations
    }

    /// Whether the text parses AND depends on no recorded deviation: what a tool written
    /// from the specification's BNF alone would also accept, as far as this parser can
    /// tell (see [`Parse::deviations`] on restrictions).
    #[must_use]
    pub fn is_spec_conformant(&self) -> bool {
        self.errors.is_empty() && self.deviations.is_empty()
    }
}

/// The deepest nesting the parser will recurse into.
///
/// Invariant 3 is that the parser does not die on any input. A recursive-descent
/// parser dies on deeply nested input by overflowing the stack, and a stack overflow
/// ABORTS the process — it is not a panic and cannot be caught, so it is strictly
/// worse than the thing the invariant forbids. Measured on this parser, the shallowest
/// construct overflows somewhere between 2000 and 5000 levels; 400 is well inside that
/// and far past anything a person writes.
///
/// Reaching it is reported and recovered from, not silently truncated: the tokens
/// still enter the tree through the enclosing body's recovery, so the round-trip holds.
const MAX_DEPTH: u32 = 400;

/// What `membership` found after the `MemberPrefix`, which decides the member's node.
#[derive(Clone, Copy)]
enum MemberElement {
    /// A package, definition, classifier or annotating element: not a usage.
    Other,
    /// A usage of this class.
    Usage(UsageClass),
    /// An `ActionNode`, which is not a `UsageElement` at all: `ActionBehaviorMember =
    /// BehaviorUsageMember | ActionNodeMember` (`SysML` 8.2.2.17.1), so it is owned
    /// beside the behaviour usages rather than as one of them.
    ActionNode,
}

/// A case production pair: one kind keyword run over the case layer's spine.
///
/// ```text
/// CaseDefinition         = OccurrenceDefinitionPrefix 'case' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.22
/// CaseUsage              = OccurrenceUsagePrefix 'case'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.22
/// AnalysisCaseDefinition = OccurrenceDefinitionPrefix 'analysis' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.23
/// AnalysisCaseUsage      = OccurrenceUsagePrefix 'analysis'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.23
/// VerificationCaseDefinition = OccurrenceDefinitionPrefix 'verification' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.24
/// VerificationCaseUsage  = OccurrenceUsagePrefix 'verification'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.24
/// UseCaseDefinition      = OccurrenceDefinitionPrefix 'use' 'case' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.25
/// UseCaseUsage           = OccurrenceUsagePrefix 'use' 'case'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.25
/// ```
///
/// "An analysis case definition or usage is declared as a case definition or usage ...
/// using the kind keyword analysis" (7.23.2, receipt 2aa2d6ce), a verification case "using
/// the kind keyword verification" (7.24.2, receipt d518fc8c), and a use case "using the
/// kind keyword use case" (7.25.2, receipt 9be3712a), so the pairs differ in the keywords
/// and the metaclass alone. These four are every case definition and usage pair 8.2.2
/// states; `IncludeUseCaseUsage` (8.2.2.25) is a case usage too, but has no definition and
/// a declaration of its own, so it is read by `include_use_case_usage`.
#[derive(Clone, Copy)]
struct Case {
    /// The kind keywords in order, before the `def` of a definition: one, or `use case`'s
    /// two. The runs are disjoint on their FIRST word — `case` alone never begins
    /// `use case` — which is what lets the recognisers match a row from its start.
    keywords: &'static [&'static str],
    /// The node the definition production builds.
    definition: SyntaxKind,
    /// The node the usage production builds.
    usage: SyntaxKind,
}

/// Every case production pair read. The first keywords are reserved and disjoint, so the
/// order decides nothing.
const CASES: [Case; 4] = [
    Case {
        keywords: &["case"],
        definition: SyntaxKind::CaseDefinition,
        usage: SyntaxKind::CaseUsage,
    },
    Case {
        keywords: &["analysis"],
        definition: SyntaxKind::AnalysisCaseDefinition,
        usage: SyntaxKind::AnalysisCaseUsage,
    },
    Case {
        keywords: &["verification"],
        definition: SyntaxKind::VerificationCaseDefinition,
        usage: SyntaxKind::VerificationCaseUsage,
    },
    Case {
        keywords: &["use", "case"],
        definition: SyntaxKind::UseCaseDefinition,
        usage: SyntaxKind::UseCaseUsage,
    },
];

struct Parser<'a> {
    source: &'a str,
    /// The grammar this text is read against, chosen once by the caller (ADR-0014).
    language: Language,
    tokens: Vec<Token>,
    /// The indices into `tokens` of every token lookahead sees while comments are trivia,
    /// ascending. `peek_nth` finds the cursor in it by binary search and steps `n` from
    /// there, so asking for the `n`th meaningful token costs O(log len) rather than a
    /// filter from the cursor. A recogniser that walks by index was quadratic in the
    /// length of what it walked while it did not.
    meaningful: Vec<usize>,
    /// The same, while `REGULAR_COMMENT` is significant (`with_significant_comments`).
    meaningful_with_comments: Vec<usize>,
    /// The last cursor located in an index: `(pos, comments_significant, position)`.
    /// Most lookahead asks a step or two ahead many times at one cursor, and the binary
    /// search alone made error recovery several times slower than the filter it replaced.
    cursor: Cell<(usize, bool, usize)>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<Diagnostic>,
    /// The `PARSE-DEVIATION` notes, kept apart from `errors` (ADR-0022).
    deviations: Vec<Diagnostic>,
    /// How many nesting levels of recursive production this parser is inside.
    ///
    /// Bounded by [`MAX_DEPTH`]. Invariant 3 is that the parser does not die on any
    /// input, and a recursive-descent parser on deeply nested input dies by
    /// overflowing the stack — which aborts the process rather than panicking, so it
    /// is not even catchable. Counting the levels is what keeps that from happening.
    depth: u32,
    /// Whether the depth limit has already been reported, so it is said once rather
    /// than once per level.
    depth_reported: bool,
    /// Whether a `REGULAR_COMMENT` is a token here rather than trivia.
    ///
    /// `KerML` 8.2.2.2 makes `/* ... */` a token, and `Comment`, `Documentation` and
    /// `TextualRepresentation` take it as their body (`SysML` 8.2.2.4.2, 8.2.2.4.3).
    /// Inside a braced `RelationshipBody` every regular comment is therefore either an
    /// annotation's body or an error, never text to skip past. At a member position a
    /// bare one is a `Comment` member (see `at_bare_comment_member`), and anywhere else
    /// `eat_trivia` reports it: the Pilot hides only `WS`, `ML_NOTE` and `SL_NOTE`
    /// (KerMLExpressions.xtext:29), never `REGULAR_COMMENT`.
    comments_significant: bool,
    /// The `depth` of the `RequirementBody` a `FramedConcernUsage`'s reference alternative
    /// is reading, while it reads it, and `None` otherwise.
    ///
    /// Deviation `FramedConcernUsage` gives that alternative a `RequirementBody` where the
    /// printed clause gives a `CalculationBody` (`SysML` 8.2.2.21.1). The two share every
    /// item but `RequirementBodyItem`'s six own members, so a note falls on each of those
    /// six read at exactly this depth: `note_framed_concern_body_item` says so, and a
    /// body nested in one reads at a greater depth and draws none.
    framed_concern_body: Option<u32>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, language: Language) -> Self {
        let tokens = tokenize(source);
        let indices = |keep: fn(SyntaxKind) -> bool| -> Vec<usize> {
            tokens
                .iter()
                .enumerate()
                .filter(|(_, token)| keep(token.kind))
                .map(|(i, _)| i)
                .collect()
        };
        let meaningful = indices(|kind| !is_trivia(kind));
        let meaningful_with_comments =
            indices(|kind| !is_trivia(kind) || kind == SyntaxKind::RegularComment);
        Self {
            source,
            language,
            tokens,
            meaningful,
            meaningful_with_comments,
            cursor: Cell::new((0, false, 0)),
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            deviations: Vec::new(),
            depth: 0,
            depth_reported: false,
            comments_significant: false,
            framed_concern_body: None,
        }
    }

    fn text_of(&self, token: Token) -> &'a str {
        token.text(self.source).unwrap_or("")
    }

    /// Whether the keyword deciding which `PackageBodyElement` this is, is `text`.
    ///
    /// Looks past an optional `VisibilityIndicator`. `MemberPrefix`'s visibility is
    /// optional and `Import`'s is required, so the indicator never decides on its
    /// own — the keyword after it does.
    fn at_element_keyword(&self, text: &str) -> bool {
        self.nth_is_keyword(usize::from(self.at_visibility()), text)
    }

    /// Whether a `Package` starts at the `n`th meaningful token: its
    /// `PrefixMetadataMember*` (`SysML` 8.2.2.5.1) looked past, then `package`.
    fn at_package(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_prefix_metadata(n), "package")
    }

    /// Whether a `LibraryPackage` starts at the `n`th meaningful token: `'standard'?
    /// 'library'`, its `PrefixMetadataMember*` looked past as `at_package` looks past
    /// them, then `package` (`SysML` 8.2.2.5.1, `KerML` 8.2.5.13). The `standard` is
    /// optional by deviation `LibraryPackage` (`follow_xtext`).
    fn at_library_package(&self, n: usize) -> bool {
        let k = n + usize::from(self.nth_is_keyword(n, "standard"));
        self.nth_is_keyword(k, "library") && self.at_package(k + 1)
    }

    /// Whether a `ConstraintDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'constraint' 'def'` (`SysML` 8.2.2.20). Only the
    /// `def` separates it from a `ConstraintUsage`.
    fn at_constraint_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "constraint") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `CalculationDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'calc' 'def'` (`SysML` 8.2.2.19). Only the `def`
    /// separates it from a `CalculationUsage`.
    fn at_calculation_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "calc") && self.nth_is_keyword(after + 1, "def")
    }

    /// Which case definition starts at the `n`th meaningful token, if one does.
    ///
    /// `OccurrenceDefinitionPrefix`, a `CASES` keyword and `def` (`SysML` 8.2.2.22,
    /// 8.2.2.23). The prefix skipped is the one `case_definition` reads.
    fn at_case_definition(&self, n: usize) -> Option<Case> {
        let after = self.skip_occurrence_definition_prefix(n);
        CASES.into_iter().find(|case| {
            self.case_keywords_at(after, case)
                .is_some_and(|k| self.nth_is_keyword(k, "def"))
        })
    }

    /// The index just past `case`'s kind keywords when all of them are written from the
    /// `n`th token, in order.
    fn case_keywords_at(&self, n: usize, case: &Case) -> Option<usize> {
        let mut k = n;
        for word in case.keywords {
            if !self.nth_is_keyword(k, word) {
                return None;
            }
            k += 1;
        }
        Some(k)
    }

    /// Which case usage starts at the `n`th meaningful token, if one does.
    ///
    /// `OccurrenceUsagePrefix` and a `CASES` keyword run with no `def` after it, which is
    /// what makes it the definition beside it. The prefix skipped is the one `case_usage`
    /// reads with `occurrence_usage_prefix`. A `use case` is never taken for a `case`: `use`
    /// is not in the prefix, so the `case` after it is never at the position asked.
    fn at_case_usage(&self, n: usize) -> Option<Case> {
        let after = self.skip_occurrence_usage_prefix(n);
        CASES.into_iter().find(|case| {
            self.case_keywords_at(after, case)
                .is_some_and(|k| !self.nth_is_keyword(k, "def"))
        })
    }

    /// Whether the trailing `ResultExpressionMember` starts here rather than one more
    /// `CalculationBodyItem`.
    ///
    /// `CalculationBodyPart = CalculationBodyItem* ResultExpressionMember?`
    /// (`SysML` 8.2.2.19). The star is greedy and the expression is last, so the
    /// question is only ever "can an item start here", and everything else is the
    /// expression.
    ///
    /// The Pilot answers it with a `=>` syntactic predicate on the star. That is an LL
    /// steering device and is NOT ported (`.claude/rules/grammar.md`); what follows is
    /// the same decision made by lookahead.
    ///
    /// Every implemented item but one is introduced by a keyword — `import`, `alias`,
    /// an annotating keyword, a definition keyword, one of the seven usage keywords, or
    /// `ref` — and a keyword is not a name (`SysML` 8.2.2.1.2), so none of them can open an
    /// expression. `DefaultReferenceUsage` is the one that can: it opens on a bare name,
    /// and so does an expression. That single collision is what
    /// `usage_completion_follows` resolves.
    fn at_result_expression(&self) -> bool {
        if self.language == Language::KerMl {
            return self.at_kerml_result_expression();
        }
        let n = usize::from(self.at_visibility());
        if self.nth_is(n, SyntaxKind::At) {
            // `@T` is both a MetadataUsage (8.2.2.27) and a ClassificationExpression with
            // no left operand (KerML 8.2.5.8.1). A metadata usage ends in a MetadataBody,
            // `;` or braced, before the enclosing body closes; the expression reaches the
            // `}`. The same test the bare-name collision below makes, with the same limit:
            // an expression holding a BodyExpression reaches a `{` first and is misread
            // (`@T and x->forAll { ... }`), which `usage_completion_follows` records.
            return !self.usage_completion_follows(n);
        }
        if self.at_import()
            || self.at_element_keyword("alias")
            || self.at_annotating_member(n)
            // CalculationBodyItem = ActionBodyItem | ... (8.2.2.19), and ActionBodyItem
            // reads every usage a member does. Asked through the member's own recogniser
            // so the two cannot drift: kept as a separate list, it missed `action a;`
            // once and four productions after that.
            || self.at_sysml_keyword_member(n)
            // ActionBodyItem's action nodes, which no other body's member reaches.
            || self.at_action_node(n).is_some()
            // Asked only of a body that ends in a result expression, and
            // `ends_in_result_expression` says that is a calculation body alone.
            || self.at_source_succession_member(Body::Calculation)
        {
            return false;
        }
        if self.at_return_parameter_member()
            || ["variant", "subject", "actor", "objective"]
                .iter()
                .any(|word| self.at_element_keyword(word))
        {
            // `return` and `variant` continue the item run rather than ending it: both
            // are items of a calculation body (8.2.2.19, 8.2.2.17.1), and `subject`,
            // `actor` and `objective` are items of a case body (8.2.2.22). None is in
            // `at_sysml_keyword_member`, because none is a member `membership` reads,
            // and a reserved keyword is never an expression (8.2.2.1.2). Where a body
            // admits one, the loop asks about it first and never reaches here with it in
            // front; where a body does not (`subject` in a calculation body), this sends it
            // to recovery as unexpected text rather than to the expression reader. Either
            // way the answer does not depend on where it is asked, which is the trap
            // `membership`'s classifier guard is written against.
            return false;
        }
        if self.at_default_reference_usage(n) {
            return !self.usage_completion_follows(n);
        }
        // Not an item at all: a literal, a parenthesis, a prefix operator. Without this
        // the body loop would recover over it, which is how `{ 1 + 1 }` became three
        // errors instead of one expression.
        true
    }

    /// `KerML`'s answer to `at_result_expression`, for `FunctionBodyPart`
    /// (8.2.5.7.1): whether what is here ends the `( TypeBodyElement |
    /// ReturnFeatureMember )*` run and is the `ResultExpressionMember`.
    ///
    /// Every item but one opens on something no expression opens on: `import`, `alias`,
    /// `member`, `return`, an annotating keyword, a `NonFeatureElement`'s keyword, a
    /// `FeaturePrefix` keyword or `#`, a feature element's reserved word (8.2.2.6). The
    /// one that does not is a keywordless `Feature` (8.2.4.3.1) opening on a NAME, as an
    /// expression may, or on `~`, its `ConjugationPart` and the unary operator of table 6
    /// (8.2.5.8.1). A feature ends in a `TypeBody`, `;` or braced, before the function
    /// body closes, and an expression reaches its `}`: `usage_completion_follows`,
    /// `SysML`'s same test for the same collision. `@` is a `MetadataFeature` and a
    /// `ClassificationExpression` alike, settled the same way.
    fn at_kerml_result_expression(&self) -> bool {
        let n = usize::from(self.at_visibility());
        if self.nth_is(n, SyntaxKind::At) {
            return !self.usage_completion_follows(n);
        }
        if self.nth_is_keyword(n, "member")
            || self.nth_is_keyword(n, "return")
            || self.at_annotating_member(n)
            || self.at_kerml_non_feature_element(n)
            || self.at_kerml_keyword_feature_element(n)
        {
            return false;
        }
        if self.at_feature(n) {
            return (self.nth_is_name(n) || self.nth_is(n, SyntaxKind::Tilde))
                && !self.usage_completion_follows(n);
        }
        true
    }

    /// Whether a `RequirementDefinition` starts at the `n`th meaningful token.
    ///
    /// Its own question rather than a row in `SIMPLE_DEFINITIONS`, because it is not on
    /// that spine: it takes a `DefinitionDeclaration` and a `RequirementBody` directly,
    /// where the eight take a `Definition` — which is a declaration and a
    /// `DefinitionBody`. The prefix is the same one a part takes, so only `requirement`
    /// followed by `def` decides; `requirement r;` is a `RequirementUsage` and
    /// unimplemented.
    fn at_requirement_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "requirement") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether an implemented element of this grammar's member starts at the `n`th token.
    ///
    /// `PackageMember = MemberPrefix ( DefinitionElement | UsageElement )`
    /// (`SysML` 8.2.2.6.1), and a definition body admits usages too, which is what
    /// makes `part def Vehicle { part engine : Engine; }` one definition holding one
    /// usage. The `UsageElement`s implemented are those in `SIMPLE_USAGES`.
    ///
    /// `KerML` reaches a different set entirely (8.2.3.4.1):
    ///
    /// ```text
    /// NamespaceMember        = NonFeatureMember | NamespaceFeatureMember
    /// NonFeatureMember       = MemberPrefix MemberElement
    /// MemberElement          = AnnotatingElement | NonFeatureElement
    /// NamespaceFeatureMember = MemberPrefix FeatureElement
    /// ```
    ///
    /// Every one of `NonFeatureElement`'s alternatives is implemented: `Package` and
    /// `LibraryPackage`, shared units, the same productions in both grammars, and
    /// `KerML`'s own `Dependency`, `Namespace`, `Type`, the classifiers, `Function`,
    /// `Predicate`, `Multiplicity` and the nine relationship declarations. All ten of
    /// `FeatureElement`'s are.
    ///
    /// This is the check that stops a `SysML` construct being read out of a `KerML`
    /// file. `part def` is not reachable from `NamespaceBodyElement`, so a `.kerml` file
    /// containing one must not parse — which it silently did before ADR-0014 was
    /// implemented here, because one root was applied to both file kinds.
    fn at_member_element(&self, n: usize) -> bool {
        // Both grammars reach AnnotatingElement from their member, by different routes:
        // MemberElement = AnnotatingElement | NonFeatureElement in KerML 8.2.3.4.1, and
        // DefinitionElement's third alternative in SysML 8.2.2.6.1. So it is admitted in
        // every body either language has, and is asked before the languages part.
        if self.at_annotating_member(n) {
            return true;
        }
        match self.language {
            Language::KerMl => {
                self.at_kerml_non_feature_element(n)
                    || self.at_feature(n)
                    || self.at_kerml_keyword_feature_element(n)
            }
            Language::SysMl => {
                self.at_sysml_keyword_member(n)
                    // Only when no keyword usage starts here; see `membership`.
                    || self.at_default_reference_usage(n)
            }
        }
    }

    /// Whether a `SysML` member that opens on a KEYWORD starts at the `n`th token.
    ///
    /// Every member `at_member_element` admits in `SysML` except `DefaultReferenceUsage`,
    /// the one that opens on a bare name. Split out because a calculation body asks the
    /// same question — "can an item start here, or is this the result expression?" — and
    /// a keyword is never an expression (8.2.2.1.2), so the keyword members are exactly
    /// the items it can decide on at once. ONE LIST, so that a production added here is
    /// an item of a calculation body too. When `at_result_expression` kept a list of its
    /// own, `SuccessionAsUsage`, `BindingConnectorAsUsage`, `AssertConstraintUsage` and
    /// `CalculationUsage` were each added here and not there, and `first a then b;` in a
    /// `constraint def` body was read as the start of its result expression.
    fn at_sysml_keyword_member(&self, n: usize) -> bool {
        self.at_definition_element(n)
            || self.at_action_usage(n)
            || self.at_state_usage(n)
            || self.at_exhibit_state_usage(n)
            || self.at_perform_action_usage(n)
            || self.at_flow_usage(n)
            || self.at_succession_flow_usage(n)
            || self.at_message(n)
            || self.at_connection_usage(n)
            || self.at_interface_usage(n)
            || self.at_allocation_usage(n)
            || self.at_view_usage(n)
            || self.at_event_occurrence_usage(n)
            || self.at_individual_or_portion_usage(n).is_some()
            || self.at_succession_as_usage(n)
            || self.at_binding_connector_as_usage(n)
            || self.at_assert_constraint_usage(n)
            || self.at_satisfy_requirement_usage(n)
            || self.at_constraint_usage(n)
            || self.at_requirement_usage(n)
            || self.at_concern_usage(n)
            || self.at_viewpoint_usage(n)
            || self.at_calculation_usage(n)
            || self.at_case_usage(n).is_some()
            || self.at_include_use_case_usage(n)
            || self.at_simple_usage(n).is_some()
            || self.at_reference_usage(n)
            || self.at_extended_usage(n)
    }

    // -- productions ------------------------------------------------------------

    // production: RootNamespace@kerml
    // production: RootNamespace@sysml
    //
    // RootNamespace = PackageBodyElement*                          (SysML 8.2.2.5.1)
    // RootNamespace = NamespaceBodyElement*                        (KerML 8.2.3.4.1)
    //
    // The one production the two grammars state differently at the start symbol, which
    // is what ADR-0014 exists for. `Body::Root` carries the difference; the loop below
    // is shared, because everything else about reading a body is. Two grammar units, and
    // this reads both bodies, so it carries both markers.
    fn root_namespace(mut self) -> (GreenNode, Vec<Diagnostic>, Vec<Diagnostic>) {
        self.start_node(SyntaxKind::RootNamespace);
        self.body_elements(None, Body::Root);
        // Trailing trivia belongs to the tree as much as anything else.
        self.eat_trivia();
        self.finish_node();
        (self.builder.finish(), self.errors, self.deviations)
    }

    /// The elements of one `body`, up to `until` or end of input.
    ///
    /// `PackageBodyElement = PackageMember | ElementFilterMember | AliasMember |
    /// Import` (`SysML` 8.2.2.5.1). All four are implemented.
    ///
    /// `NamespaceBodyElement = NamespaceMember | AliasMember | Import`
    /// (`KerML` 8.2.3.4.1). All three are implemented, though `NamespaceMember`
    /// reaches far less than its `SysML` counterpart — see `at_member_element`.
    ///
    /// `DefinitionBodyItem = DefinitionMember | VariantUsageMember |
    /// NonOccurrenceUsageMember | SourceSuccessionMember? OccurrenceUsageMember |
    /// AliasMember | Import` (`SysML` 8.2.2.6.1). `DefinitionMember`, `AliasMember`
    /// and `Import` are implemented; the usage members are not.
    ///
    /// `AliasMember` and `Import` are alternatives of all three, and stated the same
    /// way in both grammars, so they are read here without asking which body this is.
    ///
    /// Anything else is recovered over one token at a time rather than abandoning
    /// the enclosing body: an editor reparses invalid text constantly, and a body
    /// that vanishes on one bad token blanks the diagram on every keystroke.
    fn body_elements(&mut self, until: Option<SyntaxKind>, body: Body) {
        while self.at_bare_comment_member()
            || (!self.at_end() && !until.is_some_and(|kind| self.at(kind)))
        {
            let start = self.pos;
            if !self.body_element(body) {
                return;
            }
            if self.pos == start {
                // No arm consumed a token. Every recogniser in `body_element` must agree
                // with the production it dispatches to, and when one does not — it
                // accepts, the production reads nothing and `expect_*` records an error
                // without consuming — the loop asks the same question at the same token
                // for ever, and each pass adds a diagnostic until the process runs out of
                // memory. Taking one token turns such a disagreement into one diagnostic
                // (invariant 3); the disagreement itself is still the defect to fix.
                self.error_token();
            }
        }
    }

    /// One element of `body`, as `body_elements` describes them. Returns `false` when
    /// what is left is the body's trailing result expression, which ends the item run.
    // production: NamespaceBodyElement@kerml
    // production: NamespaceMember@kerml
    // production: PackageBodyElement@sysml
    // production: DefinitionBodyItem@sysml
    // production: RequirementBodyItem@sysml
    // production: ViewDefinitionBodyItem@sysml
    // production: ViewBodyItem@sysml
    // production: NonBehaviorBodyItem@sysml
    // production: ActionBodyItem@sysml
    // production: CalculationBodyItem@sysml
    // production: CaseBodyItem@sysml
    // production: StateBodyItem@sysml
    //
    // Every body's item production is an alternation over memberships, and this one
    // dispatcher reads each of them whole, parameterised by `body`: `Body::member` names
    // the membership a member is owned through, and the `admits_*` questions which of a
    // production's extra alternatives the body has. The productions are quoted at each
    // `Body` variant, and the alternations build no node: the membership says which. Two
    // tests hold the whole matrix, every alternative tried in every body, admitted with
    // its membership exactly where its production lists it and reported elsewhere:
    // `every_body_admits_exactly_its_item_production` (SysML) and
    // `every_kerml_body_admits_exactly_its_elements` (KerML).
    //
    // NonBehaviorBodyItem is reached as the first alternative of ActionBodyItem (8.2.2.17.1)
    // and StateBodyItem (8.2.2.18.1), never alone; NamespaceMember is
    // `NonFeatureMember | NamespaceFeatureMember` (KerML 8.2.3.4.1), the first read by
    // `membership`, the second by `kerml_feature_item`.
    fn body_element(&mut self, body: Body) -> bool {
        if self.depth >= MAX_DEPTH {
            // Too deeply nested to recurse into another body. Recover one token
            // at a time, exactly as unrecognised text is recovered: every byte
            // still reaches the tree, and the stack does not grow (invariant 3).
            self.report_too_deep();
            self.error_token();
        } else if self.at_bare_comment_member() {
            // A Comment member, AnnotatingElement being a MemberElement in KerML
            // (8.2.3.4.3) and a DefinitionElement in SysML (8.2.2.5.2). Before the result
            // expression too: `calc def C { /* c */ x + 1 }` is an item, then the result.
            self.bare_comment_member(body);
        } else if self.at_import() {
            self.import();
        } else if self.at_element_keyword("alias") {
            self.alias_member();
        } else if body.admits_filter(self.language) && self.at_element_keyword("filter") {
            // Where a filter is admitted is not the same question as which member
            // a body owns; see `Body`. A `filter` in a SysML definition body
            // (8.2.2.5.1 against 8.2.2.6.1) or at a KerML root (8.2.3.4.1) is
            // reported rather than accepted, and
            // tests/rejection/element-filter-member-is-not-a-definition-body-item.sysml
            // and element-filter-member-is-not-a-kerml-root-element.kerml hold those.
            self.element_filter_member();
        } else if self.language == Language::KerMl
            && body.ends_in_result_expression()
            && self.at_result_expression()
        {
            // A KerML function body's item run is over, and what is left is its result
            // expression. Asked BEFORE the features, because a keywordless feature opens
            // on a name as `age >= 35` does, and the feature branch would take it.
            return false;
        } else if self.language == Language::KerMl && self.kerml_feature_item(body) {
            // Read by the call, which answers whether it read one.
        } else if body == Body::Interface && self.at_usage_no_interface_body_admits() {
            // InterfaceNonOccurrenceUsageElement lists ReferenceUsage, AttributeUsage,
            // EnumerationUsage, BindingConnectorAsUsage and SuccessionAsUsage and no
            // other (8.2.2.14.1): a keywordless usage, or one declared by `#` alone, is no
            // item of an interface body. Reported and recovered over as a whole.
            self.recover_statement();
        } else if self.body_specific_item(body) {
            // GuardedSuccessionMember, InitialNodeMember, ReturnParameterMember,
            // VariantUsageMember, RequirementConstraintMember,
            // RequirementVerificationMember, SubjectMember, ActorMember, ObjectiveMember
            // and TransitionUsageMember: see
            // `body_specific_item`.
        } else if body.ends_in_result_expression() && self.at_result_expression() {
            // The item run is over and what is left is the body's trailing
            // expression, which is not a member. `calculation_body_part` reads it;
            // the loop must not recover over it one token at a time.
            return false;
        } else if body.admits_source_succession() && self.at_source_succession_member(body) {
            // `SourceSuccessionMember? <occurrence usage member>`, in whichever of
            // three item productions this body has; see `source_succession_item`.
            self.source_succession_item(body);
        } else if self.at_member_element(usize::from(self.at_visibility()))
            || (body.admits_action_body_item()
                && self
                    .at_action_node(usize::from(self.at_visibility()))
                    .is_some())
        {
            // An action node is an ActionNodeMember, ActionBehaviorMember's second
            // alternative (8.2.2.17.1), and so an item of the action-body family only:
            // no other body's item production reaches ActionNode, which
            // validateControlNodeOwningType states of the metaclass too (8.3.17.6,
            // receipt 695df335). Hence here, with the body in hand, and not in the
            // body-agnostic `at_member_element`.
            let element = self.membership(body);
            self.behaviour_targets(body, element);
        } else {
            self.recover_statement();
        }
        true
    }

    /// The items that own their element through a membership of their own, each told by a
    /// reserved keyword and admitted by the body's item production alone. Returns whether
    /// one was read.
    ///
    /// Split out of `body_element`, which asks it before the result-expression test: every
    /// one of these continues an item run, and none is admitted by a calculation body
    /// except the four that are (the guarded succession, `first`, `return`, `variant`),
    /// nor by a case body except those four and `subject`, `actor` and `objective`, so the others'
    /// position relative to that test decides nothing. The arms are disjoint on their keywords, so
    /// their order decides nothing either.
    fn body_specific_item(&mut self, body: Body) -> bool {
        if body.admits_action_body_item() && self.at_guarded_succession_member() {
            // ActionBodyItem's fourth alternative (SysML 8.2.2.17.1). An item of its
            // own with no suffix, so it is not `initial_node_item`'s shape: nothing
            // follows it here, and tests/rejection/guarded-succession-takes-no-target-succession.sysml
            // holds that. Disjoint from the initial node below on the token after the
            // source name, so the order of the two arms decides nothing.
            self.guarded_succession_member();
        } else if body.admits_action_body_item() && self.at_initial_node_member() {
            // ActionBodyItem's second alternative (SysML 8.2.2.17.1), reached from an
            // action body and, through CalculationBodyItem (8.2.2.19), a calculation
            // body. Owns its membership as ReturnParameterMember below does, so it
            // cannot go through `membership`. Before the result-expression test for
            // the same reason `return` is: it continues the item run.
            self.initial_node_item();
        } else if body.admits_return_parameter() && self.at_return_parameter_member() {
            // CalculationBodyItem's second alternative (SysML 8.2.2.19). Before the
            // result-expression test below, because `return` is where the item run
            // continues rather than where it ends; `at_result_expression` says so
            // too, so neither position depends on the other being right.
            if body == Body::Case {
                // 8.2.2.22's CaseBodyItem has no ReturnParameterMember; see
                // `Body::admits_return_parameter`.
                // deviation: CaseBodyItem
                self.note_deviation("CaseBodyItem", "a return parameter in a case body");
            }
            self.return_parameter_member();
        } else if body.admits_variant() && self.at_element_keyword("variant") {
            // DefinitionBodyItem's and NonBehaviorBodyItem's VariantUsageMember
            // (SysML 8.2.2.6.1, 8.2.2.17.1). Owns its element through a VariantMembership
            // of its own, so it cannot go through `membership`. `variant` is reserved,
            // so it decides; before the result-expression test, as `return` is, because
            // it continues the item run.
            self.variant_usage_member();
        } else if self.requirement_body_member(body) {
            // SubjectMember, RequirementConstraintMember, FramedConcernMember,
            // RequirementVerificationMember, ActorMember and StakeholderMember: see
            // `requirement_body_member`.
        } else if body.admits_expose() && self.at_keyword("expose") {
            // ViewBodyItem's Expose (SysML 8.2.2.26.2). `at_keyword`, not
            // `at_element_keyword`: an Expose writes no visibility, so a `private` before
            // it is no item and is recovered over.
            self.expose();
        } else if body.admits_render() && self.at_element_keyword("render") {
            // ViewDefinitionBodyItem's and ViewBodyItem's ViewRenderingMember (SysML
            // 8.2.2.26.1, .2): a ViewRenderingMembership of its own, so not `membership`'s.
            self.view_rendering_member();
        } else if body.admits_objective() && self.at_element_keyword("objective") {
            // CaseBodyItem's fourth alternative (SysML 8.2.2.22), owning its requirement
            // through an ObjectiveMembership of its own, as SubjectMember does.
            self.objective_member();
        } else if body.admits_state_action()
            && ["entry", "do", "exit"]
                .iter()
                .any(|word| self.at_element_keyword(word))
        {
            // StateBodyItem's fourth, fifth and sixth alternatives (SysML 8.2.2.18.1),
            // each owning a StateActionUsage through a StateSubactionMembership of its
            // own; an entry action takes its EntryTransitionMembers after it.
            self.state_action_item();
        } else if body.admits_transition() && self.at_element_keyword("transition") {
            // StateBodyItem's third alternative (SysML 8.2.2.18.1). An item of its own,
            // owning its element through a membership of its own. A `transition` that
            // opens a TARGET transition is read as a suffix by `behaviour_targets` and
            // never reaches here unless nothing precedes it, where it is no item and
            // `transition_usage` reports the missing source.
            self.transition_usage_member();
        } else {
            return false;
        }
        true
    }

    /// `RequirementBodyItem`'s six members of its own (`SysML` 8.2.2.21.1), each owning its
    /// element through a membership of its own. Returns whether one was read. Split out of
    /// `body_specific_item` for clippy's complexity budget; the arms are disjoint on their
    /// keywords, so where they are asked decides nothing.
    fn requirement_body_member(&mut self, body: Body) -> bool {
        if body.admits_requirement_constraint()
            && (self.at_element_keyword("require") || self.at_element_keyword("assume"))
        {
            // RequirementBodyItem's third alternative (SysML 8.2.2.21.1). Owns its
            // element through RequirementConstraintMembership, so like SubjectMember
            // below it cannot go through `membership`.
            self.note_framed_concern_body_item(body);
            self.requirement_constraint_member();
        } else if body.admits_requirement_verification() && self.at_element_keyword("verify") {
            // RequirementBodyItem's fifth alternative (SysML 8.2.2.21.1), owning its
            // requirement through a RequirementVerificationMembership of its own.
            self.note_framed_concern_body_item(body);
            self.requirement_verification_member();
        } else if body.admits_subject() && self.at_element_keyword("subject") {
            // RequirementBodyItem's second alternative (SysML 8.2.2.21.1). Like
            // NamespaceFeatureMember above, it owns its element through a membership
            // of its own — SubjectMembership — so it cannot go through `membership`,
            // which builds the body's ordinary member node.
            self.note_framed_concern_body_item(body);
            self.subject_member();
        } else if body.admits_actor() && self.at_element_keyword("actor") {
            // RequirementBodyItem's sixth alternative (SysML 8.2.2.21.1) and CaseBodyItem's
            // third (8.2.2.22): an ActorMembership of its own, as SubjectMember is.
            self.note_framed_concern_body_item(body);
            self.actor_member();
        } else if body.admits_stakeholder() && self.at_element_keyword("stakeholder") {
            // RequirementBodyItem's seventh alternative (SysML 8.2.2.21.1): a
            // StakeholderMembership of its own, as ActorMember is.
            self.note_framed_concern_body_item(body);
            self.stakeholder_member();
        } else if body.admits_framed_concern() && self.at_element_keyword("frame") {
            // RequirementBodyItem's fourth alternative (SysML 8.2.2.21.1): a
            // FramedConcernMembership of its own, as RequirementConstraintMember is.
            self.note_framed_concern_body_item(body);
            self.framed_concern_member();
        } else {
            return false;
        }
        true
    }

    /// Whether a KEYWORD that begins a member is written here.
    ///
    /// The recogniser is the body loop's, minus everything that opens on a bare NAME: a
    /// `ReferenceUsage` needs its `ref`, and `DefaultReferenceUsage` is excluded outright,
    /// because a NAME is exactly what a run of unreadable text is full of. Asked only
    /// while recovering, where the question is "does a new member start here", not "which
    /// member is it".
    fn at_member_keyword(&self) -> bool {
        self.at_import()
            || self.at_element_keyword("alias")
            || self.at_element_keyword("filter")
            || self.at_annotating_member(0)
            || match self.language {
                Language::KerMl => {
                    self.at_package(0)
                        || self.at_library_package(0)
                        || self.at_dependency(0)
                        || self.at_classifier(0).is_some()
                        || self.at_kerml_keyword_feature_element(0)
                        || self.at_relationship_declaration(0).is_some()
                        || self.at_kerml_type(0)
                        || self.at_kerml_function(0).is_some()
                        || self.at_kerml_namespace(0)
                        || self.nth_is_keyword(0, "multiplicity")
                }
                Language::SysMl => {
                    self.at_definition_element(0)
                        // A body item rather than a member `membership` reads, and SysML's
                        // alone: KerML has no variants.
                        || self.at_element_keyword("variant")
                        || self.at_simple_usage(0).is_some()
                        || self.at_action_usage(0)
                        || self.at_state_usage(0)
                        || self.at_exhibit_state_usage(0)
                        // StateBodyItems rather than members `membership` reads.
                        || ["transition", "entry", "do", "exit"]
                            .iter()
                            .any(|word| self.at_element_keyword(word))
                        || self.at_perform_action_usage(0)
                        || self.at_flow_usage(0)
                        || self.at_succession_flow_usage(0)
                        || self.at_connection_usage(0)
                        || self.at_succession_as_usage(0)
                        || self.at_binding_connector_as_usage(0)
                        || self.at_assert_constraint_usage(0)
                        || self.at_constraint_usage(0)
                        || self.at_requirement_usage(0)
                        || self.at_calculation_usage(0)
                        || self.at_case_usage(0).is_some()
                        || self.at_include_use_case_usage(0)
                        // Body items rather than members `membership` reads.
                        || self.at_element_keyword("verify")
                        || self.at_element_keyword("actor")
                        || self.at_element_keyword("objective")
                        || self.at_action_node(0).is_some()
                }
            }
    }

    /// The suffix after a behaviour usage: `ActionTargetSuccessionMember*` in an action
    /// body, `TargetTransitionUsageMember*` in a state body.
    ///
    /// `ActionBodyItem`'s third alternative is `SourceSuccessionMember?
    /// ActionBehaviorMember ActionTargetSuccessionMember*` (8.2.2.17.1), so the `then X;`
    /// members belong to the item just read when — and only when — it was an
    /// `ActionBehaviorMember`, a `BehaviorUsageMember` or an `ActionNodeMember`; the
    /// `NonBehaviorBodyItem` alternative that reads structure usages takes no such
    /// suffix, so `part p; then b;` leaves the `then` reported.
    fn behaviour_targets(&mut self, body: Body, element: MemberElement) {
        if body == Body::State && matches!(element, MemberElement::Usage(UsageClass::Behavior)) {
            // StateBodyItem: `SourceSuccessionMember? BehaviorUsageMember
            // TargetTransitionUsageMember*` (8.2.2.18.1). A target transition's source is
            // "the closest lexically previous state usage" (7.18.3, receipt 6e6e9493),
            // connected in resolution.
            while self.at_target_transition_usage_member() {
                self.target_transition_usage_member();
            }
            return;
        }
        if body.admits_action_body_item()
            && matches!(
                element,
                MemberElement::Usage(UsageClass::Behavior) | MemberElement::ActionNode
            )
        {
            while self.at_action_target_succession_member() {
                self.action_target_succession_member();
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

    // production: NonFeatureMember
    //
    // NonFeatureMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += MemberElement          (KerML 8.2.3.4.1)
    //
    // The KerML member, and the third production this one method builds. It has the
    // same shape as PackageMember and DefinitionMember — a MemberPrefix and one owned
    // element — and differs only in which elements it may own, which `at_member_element`
    // decides. NamespaceMember gets no node, as DefinitionElement and UsageElement get
    // none: it is an alternation, and the alternative that matched says which was taken.
    //
    // NamespaceFeatureMember, its sibling, is marked at `namespace_feature_member`.
    //
    // production: NonOccurrenceUsageMember
    // production: OccurrenceUsageMember
    // production: StructureUsageMember
    // production: BehaviorUsageMember
    //
    // <kind>UsageMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += <kind>UsageElement    (SysML 8.2.2.6.1)
    //
    // The four usage memberships, with the same shape again. Which one a member is
    // depends on the element after the MemberPrefix, so the node is opened at a
    // checkpoint once the element has said what it is — `Body::member` maps the two to
    // the node. Each is marked as the member it is, as DefinitionMember is; the
    // <kind>UsageElement alternations are marked at the methods that read them
    // (`usage_element_of_class` and its class methods). BehaviorUsageMember is reached only from ActionBodyItem's third
    // alternative, whose `then` prefix and trailing target successions its callers read.
    //
    // production: ActionNodeMember@sysml
    //
    // ActionNodeMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += ActionNode            (SysML 8.2.2.17.1)
    //
    // The same shape once more, marked as the member it is; ActionNode is marked at
    // `action_node`.
    //
    // production: ActionBehaviorMember@sysml
    //
    // ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember   (SysML 8.2.2.17.1)
    //
    // An alternation with no node, marked because both of its alternatives are read —
    // the convention OwnedExpression follows. Which one was taken is the member node.
    // production: MemberElement@kerml
    // production: DefinitionElement@sysml
    //
    // MemberElement = AnnotatingElement | NonFeatureElement          (KerML 8.2.3.4.3)
    // DefinitionElement = Package | LibraryPackage | AnnotatingElement | Dependency
    //     | the twenty-five definitions                            (SysML 8.2.2.5.2)
    //
    // Both alternations, read here: the annotating element first in either language, a
    // bare REGULAR_COMMENT among them (see `at_bare_comment_member`); then KerML's
    // NonFeatureElement whole, by `kerml_non_feature_element`; and SysML's packages here
    // and the rest by `definition_element`. No node: the element says which.
    fn membership(&mut self, body: Body) -> MemberElement {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.member_prefix();
        let mut element = MemberElement::Other;
        if self.at_annotating_member(0) {
            self.annotating_element();
        } else if self.language == Language::KerMl {
            // MemberElement's other alternative (KerML 8.2.3.4.3). The guard is here
            // rather than left to `at_member_element`'s caller, because a dispatch that
            // is only correct when reached one way is a trap: every classifier unit is
            // scoped `kerml`, and SysML reaches DefinitionElement instead, so `class
            // Foo;` in a .sysml file is text SysML does not state.
            if !self.kerml_non_feature_element() {
                self.error_expected("a package, a classifier or a dependency");
            }
        } else if self.at_package(0) {
            self.package();
        } else if self.at_library_package(0) {
            self.library_package();
        } else if let Some(node) = self
            .at_action_node(0)
            .filter(|_| body.admits_action_body_item())
        {
            // Only the action-body family reaches ActionNodeMember; `body_elements`
            // asks the same question before calling here. The guard is repeated for
            // the reason the classifier one above is: a dispatch that is only correct
            // when reached one way is a trap. BEFORE the usages, because `action publish
            // send x;` opens as an ActionUsage does and is not one: the `send` after the
            // declaration is what makes it a SendNode.
            self.action_node(node);
            element = MemberElement::ActionNode;
        } else if body == Body::Interface && self.at_default_interface_end(0) {
            // InterfaceOccurrenceUsageElement's first alternative (8.2.2.14.1). Its class
            // is neither Structure nor Behavior, and `Body::member` owns every
            // occurrence class through InterfaceOccurrenceUsageMember alike; Structure is
            // the one that takes no target successions after it, as an end takes none.
            self.default_interface_end();
            element = MemberElement::Usage(UsageClass::Structure);
        } else if self.definition_element() {
            // A definition: `MemberElement::Other`, which `element` already is.
        } else if let Some(class) = self.usage_element_of_class() {
            element = MemberElement::Usage(class);
        } else {
            self.error_expected("a package, a part definition or a usage");
        }
        self.start_node_at(start, body.member(self.language, element));
        self.finish_node();
        element
    }

    /// A `RequirementDefinition`, `ConcernDefinition`, `ViewpointDefinition`,
    /// `ConstraintDefinition` or `CalculationDefinition` read as a `DefinitionElement`,
    /// returning whether one was.
    /// Split out of `definition_element` so that function stays within clippy's complexity
    /// budget; each opens on its own keyword pair, so the order decides nothing.
    fn requirement_family_definition(&mut self) -> bool {
        if self.at_requirement_definition(0) {
            self.requirement_definition();
        } else if self.at_concern_definition(0) {
            self.concern_definition();
        } else if self.at_viewpoint_definition(0) {
            self.viewpoint_definition();
        } else if self.at_constraint_definition(0) {
            self.constraint_definition();
        } else if self.at_calculation_definition(0) {
            self.calculation_definition();
        } else {
            return false;
        }
        true
    }

    /// Whether a `CalculationUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'calc'` with no `def` after it (`SysML` 8.2.2.19): the `def`
    /// is what makes it the `CalculationDefinition` beside it, as for `at_action_usage`.
    /// The prefix skipped is the one `calculation_usage` reads with
    /// `occurrence_usage_prefix`.
    fn at_calculation_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "calc") && !self.nth_is_keyword(after + 1, "def")
    }

    /// Whether an `IncludeUseCaseUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'include'` (`SysML` 8.2.2.25). Like `perform`, the keyword
    /// names a usage and nothing else, so there is no `def` to test. The prefix skipped is
    /// the one `include_use_case_usage` reads with `occurrence_usage_prefix`.
    fn at_include_use_case_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "include")
    }

    // production: ElementFilterMember
    //
    // ElementFilterMember : ElementFilterMembership =
    //     MemberPrefix 'filter' ownedRelatedElement += OwnedExpression ';'
    //                                                            (SysML 8.2.2.5.1)
    //
    // The fourth PackageBodyElement, and the one that waited on this layer. The
    // corpus idiom is a bare classification operator — `filter @Safety;` — which is
    // a ClassificationExpression with no ArgumentMember, the alternative the clause
    // makes optional.
    fn element_filter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ElementFilterMember);
        self.member_prefix();
        self.expect_keyword("filter");
        self.owned_expression();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish_node();
    }

    // production: RequirementDefinition
    //
    // RequirementDefinition = OccurrenceDefinitionPrefix 'requirement' 'def'
    //                         DefinitionDeclaration RequirementBody
    //                                                            (SysML 8.2.2.21.1)
    //
    // Not in SIMPLE_DEFINITIONS, and what keeps it out is the END rather than the
    // prefix: the eight take `Definition`, which is `DefinitionDeclaration
    // DefinitionBody`, and this one names the declaration and the body separately. So
    // there is no `Definition` node in a requirement's tree, and reading it with the
    // shared spine would both invent that node and read the body against the wrong rule.
    //
    // An OccurrenceDefinitionPrefix, the same one a part takes, so `individual
    // requirement def R;` is a requirement rather than an error.
    //
    // The metaclass is SysML::RequirementDefinition (8.3.21.8), a ConstraintDefinition.
    // checkRequirementDefinitionSpecialization says every one of them directly or
    // indirectly specializes `Requirements::RequirementCheck` from the Systems Model
    // Library. That is an IMPLIED SPECIALIZATION and does not belong here: it injects no
    // tokens and sv2-hir is where it goes (ADR-0002). The concrete syntax reifies
    // nothing beyond the declaration and the body — every derived attribute on the
    // metaclass is computed from memberships the body supplies — which is what makes
    // this unlike PortDefinition, whose trailing member consumes no tokens and is
    // still part of the production.
    //
    // The Pilot factors 'requirement' 'def' into RequirementDefKeyword; deviations.json
    // records that as xtext_only/follow_spec, so the literals are matched here directly.
    fn requirement_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("requirement");
        self.expect_keyword("def");
        self.definition_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ConcernDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'concern' 'def'` (`SysML` 8.2.2.21.3), as
    /// `at_requirement_definition` asks of `requirement`.
    fn at_concern_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "concern") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ConcernDefinition@sysml
    //
    // ConcernDefinition =
    //     OccurrenceDefinitionPrefix 'concern' 'def'
    //     DefinitionDeclaration RequirementBody                  (SysML 8.2.2.21.3)
    //
    // "A concern definition or usage is declared as a requirement definition or usage (see
    // 7.21.2 ) using the kind keyword concern instead of requirement. Otherwise, a concern
    // definition or usage is specified exactly like a regular requirement definition or
    // usage" (7.21.3, receipt 0e50a373). RequirementDefinition's spine with the other
    // keyword; the Pilot factors it into ConcernDefKeyword, deviation ConcernDefKeyword
    // (xtext_only, follow_spec). The metaclass is ConcernDefinition (8.3.21.3, receipt
    // 35e9eaca), a RequirementDefinition.
    //
    // implied specialization: Requirements::ConcernCheck
    // constraint: ConcernDefinition::checkConcernDefinitionSpecialization (8.3.21.3). An
    //     injection, so sv2-hir's (ADR-0002).
    fn concern_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConcernDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("concern");
        self.expect_keyword("def");
        self.definition_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ViewpointDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'viewpoint' 'def'` (`SysML` 8.2.2.26.3), as
    /// `at_concern_definition` asks of `concern`.
    fn at_viewpoint_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "viewpoint") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewpointDefinition@sysml
    //
    // ViewpointDefinition =
    //     OccurrenceDefinitionPrefix 'viewpoint' 'def'
    //     DefinitionDeclaration RequirementBody                  (SysML 8.2.2.26.3)
    //
    // "A viewpoint definition or usage is declared as a kind of requirement definition or
    // usage" (7.26.3, receipt 1813f74a), so RequirementDefinition's spine with the keyword
    // `viewpoint`, as ConcernDefinition is. The Pilot factors the keywords into
    // ViewpointDefKeyword, deviation ViewpointDefKeyword (xtext_only, follow_spec), so the
    // literals are matched here directly. The metaclass is ViewpointDefinition (8.3.26.8,
    // receipt 3cf0e409), a RequirementDefinition.
    //
    // "The subject of a viewpoint definition or usage must be a view" (7.26.3) is a
    // semantic constraint on the subject's type, not a production: any RequirementBody
    // item is read, and checking the subject belongs above this layer (ADR-0002).
    //
    // implied specialization: Views::Viewpoint
    // constraint: ViewpointDefinition::checkViewpointDefinitionSpecialization
    //     `specializesFromLibrary('Views::Viewpoint')` (8.3.26.8). An injection, so
    //     sv2-hir's (ADR-0002).
    fn viewpoint_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewpointDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("viewpoint");
        self.expect_keyword("def");
        self.definition_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ViewDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'view' 'def'` (`SysML` 8.2.2.26.1). `view` is reserved,
    /// so a `viewpoint` is a different token and never answers this.
    fn at_view_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "view") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewDefinition@sysml
    //
    // ViewDefinition =
    //     OccurrenceDefinitionPrefix 'view' 'def'
    //     DefinitionDeclaration ViewDefinitionBody               (SysML 8.2.2.26.1)
    //
    // Not on SIMPLE_DEFINITIONS' spine: it names its declaration and its own body rather
    // than taking a `Definition`. The Pilot factors the keywords into ViewDefKeyword,
    // deviation ViewDefKeyword (xtext_only, follow_spec), so the literals are matched here. The metaclass is ViewDefinition (8.3.26.7, receipt
    // ced9a812), a PartDefinition.
    //
    // implied specialization: Views::View
    // constraint: ViewDefinition::checkViewDefinitionSpecialization
    //     `specializesFromLibrary('Views::View')` (8.3.26.7). An injection, so sv2-hir's
    //     (ADR-0002).
    fn view_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("view");
        self.expect_keyword("def");
        self.definition_declaration();
        self.view_definition_body();
        self.finish_node();
    }

    // production: ViewDefinitionBody@sysml
    //
    // ViewDefinitionBody : ViewDefinition = ';' | '{' ViewDefinitionBodyItem* '}'
    //                                                            (SysML 8.2.2.26.1)
    //
    // ViewDefinitionBodyItem is marked at `body_element`: DefinitionBodyItem's six
    // alternatives and ElementFilterMember and ViewRenderingMember; see
    // `Body::ViewDefinition`.
    fn view_definition_body(&mut self) {
        self.braced_body(
            SyntaxKind::ViewDefinitionBody,
            Body::ViewDefinition,
            "`;` or `{` after a view definition declaration",
        );
    }

    // production: ViewBody@sysml
    //
    // ViewBody : ViewUsage = ';' | '{' ViewBodyItem* '}'         (SysML 8.2.2.26.2)
    //
    // ViewBodyItem is marked at `body_element`: ViewDefinitionBodyItem's alternatives and
    // Expose; see `Body::View`.
    fn view_body(&mut self) {
        self.braced_body(
            SyntaxKind::ViewBody,
            Body::View,
            "`;` or `{` after a view usage declaration",
        );
    }

    // production: ViewRenderingMember@sysml
    //
    // ViewRenderingMember : ViewRenderingMembership =
    //     MemberPrefix 'render'
    //     ownedRelatedElement += ViewRenderingUsage              (SysML 8.2.2.26.1)
    //
    // The metaclass is ViewRenderingMembership (8.3.26.10, receipt 75857b4b), a
    // FeatureMembership whose referencedRendering is the reference's target when the
    // usage has one and the usage itself otherwise.
    //
    // constraint: ViewRenderingMembership::validateViewRenderingMembershipOwningType
    //     (8.3.26.10): the owner is a view. The grammar already reaches this member from
    //     the two view bodies alone.
    fn view_rendering_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewRenderingMember);
        self.member_prefix();
        self.expect_keyword("render");
        self.view_rendering_usage();
        self.finish_node();
    }

    // production: ViewRenderingUsage@sysml
    //
    // ViewRenderingUsage : RenderingUsage =
    //       ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart?
    //       UsageBody
    //     | ( UsageExtensionKeyword* 'rendering'
    //       | UsageExtensionKeyword+ )
    //       Usage                                                (SysML 8.2.2.26.1)
    //
    // FramedConcernUsage's and RequirementConstraintUsage's shape: a rendering by
    // reference (`render asTreeDiagram;`, training/42. Views/Views Example.sysml:13), or
    // declared (`render rendering r1: R[0..1];`, examples/Simple Tests/ViewTest.sysml:32).
    // The alternatives are told apart before either begins: the second opens on the
    // keyword `rendering` or a `#`, the first on a name, and a keyword is not a name
    // (8.2.2.1.2). `( X* 'rendering' | X+ )` is "a `#` or a `rendering`", then the rest.
    fn view_rendering_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewRenderingUsage);
        if self.at_keyword("rendering") || self.at(SyntaxKind::Hash) {
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.eat_optional_keyword("rendering");
            self.usage();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
            self.usage_body();
        }
        self.finish_node();
    }

    /// Whether a `ViewUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'view'` with no `def` after it (`SysML` 8.2.2.26.2).
    fn at_view_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "view") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewUsage@sysml
    //
    // ViewUsage =
    //     OccurrenceUsagePrefix 'view'
    //     UsageDeclaration? ValuePart? ViewBody                  (SysML 8.2.2.26.2)
    //
    // A StructureUsageElement (8.2.2.6.4). The declaration is optional whole, so
    // `view { ... }` declares nothing and `view :>> columnView[1] { ... }` (training/42.
    // Views/Views Example.sysml:17) only a redefinition; it is read only when something
    // that opens one is written, as `event_occurrence_usage` reads its own, and an empty one
    // builds no node. The Pilot factors the keyword into ViewUsageKeyword, deviation
    // ViewUsageKeyword (xtext_only, follow_spec), so the literal is matched here. The metaclass is ViewUsage (8.3.26.11, receipt 6bbae03d), a PartUsage.
    //
    // implied specialization: Views::views, and Views::View::subviews when owned by a view
    // constraint: ViewUsage::checkViewUsageSpecialization and
    //     checkViewUsageSubviewSpecialization (8.3.26.11). Injections, so sv2-hir's
    //     (ADR-0002).
    fn view_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("view");
        if self.at_name()
            || self.at(SyntaxKind::Lt)
            || self.at_feature_specialization()
            || self.at_multiplicity_part()
        {
            self.usage_declaration();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.view_body();
        self.finish_node();
    }

    // production: RequirementBody
    //
    // RequirementBody : Type = ';' | '{' RequirementBodyItem* '}'
    //                                                            (SysML 8.2.2.21.1)
    //
    // RequirementBodyItem is marked at `body_element`. It is
    //
    //     DefinitionBodyItem | SubjectMember | RequirementConstraintMember
    //     | FramedConcernMember | RequirementVerificationMember | ActorMember
    //     | StakeholderMember
    //
    // — a SUPERSET of DefinitionBodyItem, and that is the whole reason this body is
    // reachable at the cost of one method. The six extra members are read, each through a
    // membership of its own, in `body_specific_item`.
    //
    // `Body::Requirement`, which when this production landed was `Body::Definition` on
    // the argument that the two would differ in nothing. They differ in one thing, and
    // it is exactly the thing the superset is: `SubjectMember` is a RequirementBodyItem
    // and not a DefinitionBodyItem, so a body that cannot tell which of the two it is
    // reads `subject s;` as a member of both. See `Body::admits_subject`.
    fn requirement_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Requirement);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a requirement definition declaration");
        }
        self.finish_node();
    }

    /// Whether a `StateDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'state' 'def'` (`SysML` 8.2.2.18.1): the `def` is what
    /// separates it from a `StateUsage`, as for `at_action_definition`.
    fn at_state_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "state") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `StateUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'state'` with no `def` after it (`SysML` 8.2.2.18.2).
    /// `exhibit state` is an `ExhibitStateUsage`, and `exhibit` is not looked past here:
    /// `at_exhibit_state_usage` answers for it.
    fn at_state_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "state") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: StateDefinition@sysml
    //
    // StateDefinition =
    //     OccurrenceDefinitionPrefix 'state' 'def'
    //     DefinitionDeclaration StateDefBody                         (SysML 8.2.2.18.1)
    //
    // "A state definition or usage is declared as an action definition or usage ..., but
    // using the keyword state instead of action" (7.18.2, receipt 42b13f63): the
    // declaration is ActionDefinition's, and the body is a state body. The metaclass is
    // StateDefinition (8.3.18.5, receipt 249b423d), an ActionDefinition.
    //
    // implied specialization: States::StateAction
    // constraint: StateDefinition::checkStateDefinitionSpecialization — an injection, so
    //     sv2-hir's (ADR-0002). deriveStateDefinitionDoAction is sv2-resolve's.
    fn state_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::StateDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("state");
        self.expect_keyword("def");
        self.definition_declaration();
        self.state_body_part(SyntaxKind::StateDefBody);
        self.finish_node();
    }

    // production: StateDefBody@sysml
    //
    // StateDefBody : StateDefinition =
    //     ';' | ( isParallel ?= 'parallel' )? '{' StateBodyItem* '}' (SysML 8.2.2.18.1)
    //
    // production: StateUsageBody@sysml
    //
    // StateUsageBody : StateUsage =
    //     ';' | ( isParallel ?= 'parallel' )? '{' StateBodyItem* '}' (SysML 8.2.2.18.2)
    //
    // Two productions with one body, over two metaclasses, so one method builds either
    // node. `parallel` stands "just before the body part" (7.18.2, receipt 42b13f63) and
    // only before braces: `state def D parallel;` is reported.
    //
    // StateBodyItem is marked at `body_element`, which reads its items under
    // `Body::State`; `Body`'s State variant says which they are.
    fn state_body_part(&mut self, node: SyntaxKind) {
        self.eat_trivia();
        self.start_node(node);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else {
            self.eat_optional_keyword("parallel");
            if self.at(SyntaxKind::LBrace) {
                self.bump();
                self.depth += 1;
                self.body_elements(Some(SyntaxKind::RBrace), Body::State);
                self.depth -= 1;
                self.expect(SyntaxKind::RBrace, "`}`");
            } else {
                self.error_expected("`;` or `{` after a state declaration");
            }
        }
        self.finish_node();
    }

    // production: StateUsage@sysml
    //
    // StateUsage =
    //     OccurrenceUsagePrefix 'state'
    //     ActionUsageDeclaration StateUsageBody                      (SysML 8.2.2.18.2)
    //
    // ActionUsage's declaration, read under its own name as CalculationUsage reads it, and
    // a state body. A BehaviorUsageElement (8.2.2.6.4). The metaclass is StateUsage
    // (8.3.18.6, receipt 57e61560), an ActionUsage. Marked although OccurrenceUsagePrefix
    // is not, as ActionUsage is.
    //
    // implied specialization: States::stateActions; States::StateAction::substates when
    //     owned by a state; States::StateAction::exclusiveStates when that state is not
    //     parallel; Parts::Part::ownedStates when owned by a part
    // constraint: StateUsage::checkStateUsageSpecialization,
    //     checkStateUsageSubstateSpecialization, checkStateUsageExclusiveStateSpecialization
    //     and checkStateUsageOwnedStateSpecialization (8.3.18.6, receipt 57e61560) —
    //     injections, so sv2-hir's.
    fn state_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::StateUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("state");
        self.action_usage_declaration();
        self.state_body_part(SyntaxKind::StateUsageBody);
        self.finish_node();
    }

    // production: TransitionUsageMember@sysml
    //
    // TransitionUsageMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += TransitionUsage        (SysML 8.2.2.18.1)
    //
    // StateBodyItem's third alternative.
    fn transition_usage_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TransitionUsageMember);
        self.member_prefix();
        self.transition_usage();
        self.finish_node();
    }

    // TransitionUsage : TransitionUsage =
    //     'transition' ( UsageDeclaration 'first' )?
    //     ownedRelationship += FeatureChainMember
    //     ownedRelationship += EmptyParameterMember
    //     ( ownedRelationship += EmptyParameterMember
    //       ownedRelationship += TriggerActionMember )?
    //     ( ownedRelationship += GuardExpressionMember )?
    //     ( ownedRelationship += EffectBehaviorMember )?
    //     'then' ownedRelationship += TransitionSuccessionMember
    //     ActionBody                                                 (SysML 8.2.2.18.3)
    //
    // production: TransitionUsage@sysml
    //
    // Marked although EffectBehaviorUsage is not: every part of this production is read,
    // the `do` effect through EffectBehaviorMember, whose send and assignment forms are
    // the action nodes that are still reported.
    //
    // "The source and target states are identified using the same keywords as for a
    // succession, first and then" (7.18.3, receipt 6e6e9493). The source is the
    // FeatureChainMember; `first` introduces it only after a declaration, so `transition
    // a then b;` names its source with no `first` at all. The group is taken when a
    // `first` stands before the statement ends, which no other part can write.
    //
    // The order is the grammar's: the accepter, then the guard, then the effect. 7.18.3's
    // OnOff4 writes `if isEnabled accept TurnOn via commPort`, against its own prose (the
    // guard is "placed between the source and target parts, after the accepter (if any)")
    // and its OnOff3 and OnOff5; the grammar is followed and that text is reported. Its
    // OnOff4 and OnOff5 also end an effect with `;` before `then`, which no effect
    // production writes (SYSML21-450). deviations.json records both, TransitionUsage,
    // follow_spec.
    //
    // The metaclass is TransitionUsage (8.3.18.9, receipt a6f32577), an ActionUsage.
    //
    // implied specialization: Actions::transitionActions, and, owned by a state with a
    //     state usage as its source, States::StateAction::stateTransitions
    // constraint: TransitionUsage::checkTransitionUsageSpecialization and
    //     checkTransitionUsageStateSpecialization (8.3.18.9, receipt a6f32577) — injections,
    //     so sv2-hir's, as are checkTransitionUsageTransitionFeatureSpecialization,
    //     checkTransitionUsagePayloadSpecialization and the two binding connectors.
    //     checkTransitionUsageActionSpecialization (Actions::Action::decisionTransitions)
    //     governs a transition owned by an action, which this grammar position is not.
    fn transition_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TransitionUsage);
        self.expect_keyword("transition");
        if self.scan_for_keyword(0, "first").is_some() {
            self.usage_declaration();
            self.expect_keyword("first");
        }
        self.sysml_feature_chain_member();
        self.empty_parameter_member();
        self.transition_trigger_guard_and_effect();
        self.expect_keyword("then");
        self.transition_succession_member();
        self.action_body();
        self.finish_node();
    }

    /// `( EmptyParameterMember TriggerActionMember )? GuardExpressionMember?
    /// EffectBehaviorMember?`, the part of a transition between its source and its `then`
    /// that both transition productions write the same way (`SysML` 8.2.2.18.3).
    fn transition_trigger_guard_and_effect(&mut self) {
        if self.at_keyword("accept") {
            self.empty_parameter_member();
            self.trigger_action_member();
        }
        if self.at_keyword("if") {
            self.guard_expression_member();
        }
        if self.at_keyword("do") {
            self.effect_behavior_member();
        }
    }

    /// Whether a `TargetTransitionUsageMember` starts here.
    ///
    /// Asked only after a behaviour usage in a state body, where it is the suffix. The
    /// production opens on an optional prefix, so it begins with one of four things:
    /// `transition` followed by an accepter, a guard, an effect or the `then`; `accept`;
    /// `if` with a
    /// `then` after it; or a bare `then`. A bare `then` is a target transition only when
    /// a `ConnectorEnd` and a body follow it, as `at_target_succession` asks: `then state
    /// s;` is the NEXT item's `SourceSuccessionMember` instead, and `transition a then
    /// b;` is a `TransitionUsageMember`, which names its source.
    fn at_target_transition_usage_member(&self) -> bool {
        let n = usize::from(self.at_visibility());
        if self.nth_is_keyword(n, "transition") {
            return ["accept", "if", "do", "then"]
                .iter()
                .any(|word| self.nth_is_keyword(n + 1, word));
        }
        self.nth_is_keyword(n, "accept")
            || (self.nth_is_keyword(n, "if") && self.scan_for_keyword(n + 1, "then").is_some())
            || (self.nth_is_keyword(n, "then")
                && self.skip_connector_end(n + 1).is_some_and(|after| {
                    self.nth_is(after, SyntaxKind::Semicolon)
                        || self.nth_is(after, SyntaxKind::LBrace)
                }))
    }

    // production: TargetTransitionUsageMember@sysml
    //
    // TargetTransitionUsageMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += TargetTransitionUsage  (SysML 8.2.2.18.1)
    fn target_transition_usage_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TargetTransitionUsageMember);
        self.member_prefix();
        self.target_transition_usage();
        self.finish_node();
    }

    // TargetTransitionUsage : TransitionUsage =
    //     ownedRelationship += EmptyParameterMember
    //     ( 'transition'
    //       ( ownedRelationship += EmptyParameterMember
    //         ownedRelationship += TriggerActionMember )?
    //       ( ownedRelationship += GuardExpressionMember )?
    //       ( ownedRelationship += EffectBehaviorMember )?
    //     | ownedRelationship += EmptyParameterMember
    //       ownedRelationship += TriggerActionMember
    //       ( ownedRelationship += GuardExpressionMember )?
    //       ( ownedRelationship += EffectBehaviorMember )?
    //     | ownedRelationship += GuardExpressionMember
    //       ( ownedRelationship += EffectBehaviorMember )?
    //     )?
    //     'then' ownedRelationship += TransitionSuccessionMember
    //     ActionBody                                                 (SysML 8.2.2.18.3)
    //
    // production: TargetTransitionUsage@sysml
    //
    // Marked, as TransitionUsage is: every part is read, the effect included.
    //
    // "A transition usage without a declaration part, in which both the transition
    // keyword and the source part can be omitted. In this case, the source is taken to be
    // the closest lexically previous state usage" (7.18.3, receipt 6e6e9493). So the
    // source is written nowhere, and the first EmptyParameterMember stands where
    // TransitionUsage's FeatureChainMember and EmptyParameterMember do. The three
    // alternatives of the group write their parts in one order, so after the optional
    // `transition` they are read as TransitionUsage reads them.
    fn target_transition_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TargetTransitionUsage);
        self.empty_parameter_member();
        self.eat_optional_keyword("transition");
        self.transition_trigger_guard_and_effect();
        self.expect_keyword("then");
        self.transition_succession_member();
        self.action_body();
        self.finish_node();
    }

    // production: EmptyParameterMember@sysml
    //
    // EmptyParameterMember : ParameterMembership =
    //     ownedRelatedElement += EmptyUsage                          (SysML 8.2.2.17.4)
    //
    // production: EmptyUsage@sysml
    //
    // EmptyUsage : ReferenceUsage = {}                               (SysML 8.2.2.17.4)
    //
    // A parameter the text never writes, built from no tokens, as EmptyEndMember is.
    // Trivia is not eaten first, or it would land inside a node the author never wrote.
    fn empty_parameter_member(&mut self) {
        self.start_node(SyntaxKind::EmptyParameterMember);
        self.start_node(SyntaxKind::EmptyUsage);
        self.finish_node();
        self.finish_node();
    }

    // production: TriggerActionMember@sysml
    //
    // TriggerActionMember : TransitionFeatureMembership =
    //     'accept' { kind = 'trigger' }
    //     ownedRelatedElement += TriggerAction                       (SysML 8.2.2.18.3)
    //
    // `{ kind = 'trigger' }` sets the TransitionFeatureMembership's kind, as
    // GuardExpressionMember's `{ kind = 'guard' }` does, and contributes no token.
    //
    // production: TriggerAction@sysml
    //
    // TriggerAction : AcceptActionUsage = AcceptParameterPart        (SysML 8.2.2.18.3)
    //
    // An AcceptActionUsage written with no `accept` of its own: the member's is the one.
    fn trigger_action_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TriggerActionMember);
        self.expect_keyword("accept");
        self.eat_trivia();
        self.start_node(SyntaxKind::TriggerAction);
        self.accept_parameter_part();
        self.finish_node();
        self.finish_node();
    }

    // production: AcceptParameterPart@sysml
    //
    // AcceptParameterPart : AcceptActionUsage =
    //     ownedRelationship += PayloadParameterMember
    //     ( 'via' ownedRelationship += NodeParameterMember )?        (SysML 8.2.2.17.4)
    //
    // "The accepter action for a transition usage is ... notated using the accept keyword,
    // with its payload and receiver parameters" (7.18.3, receipt 6e6e9493): `via` names
    // the receiver. AcceptNode reads this part too, through AcceptNodeDeclaration.
    fn accept_parameter_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AcceptParameterPart);
        self.payload_parameter_member();
        if self.at_keyword("via") {
            self.bump_as(keyword("via").unwrap_or(SyntaxKind::BasicName));
            self.node_parameter_member();
        }
        self.finish_node();
    }

    // production: PayloadParameterMember@sysml
    //
    // PayloadParameterMember : ParameterMembership =
    //     ownedRelatedElement += PayloadParameter                    (SysML 8.2.2.17.4)
    //
    // production: PayloadParameter@sysml
    //
    // PayloadParameter : ReferenceUsage =
    //       PayloadFeature
    //     | Identification PayloadFeatureSpecializationPart?
    //       TriggerValuePart                                         (SysML 8.2.2.17.4)
    //
    // The first alternative is PayloadFeature's own production, read under this node as
    // FlowPayloadFeature reads it under its own. The second is the change and time
    // triggers, `when`, `at` and `after` (7.17.8, receipt bb0d6dc7), whose payload is a
    // feature valued by the trigger. The two share their opening, and what separates them
    // is whether a trigger keyword stands before the parameter ends; see
    // `at_trigger_payload`.
    fn payload_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadParameterMember);
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadParameter);
        if self.at_trigger_payload() {
            self.identification();
            if self.at_feature_specialization() || self.at_multiplicity_part() {
                self.payload_feature_specialization_part();
            }
            self.trigger_value_part();
        } else {
            self.payload_feature();
        }
        self.finish_node();
        self.finish_node();
    }

    /// Whether the payload here is `PayloadParameter`'s trigger alternative.
    ///
    /// It is when `at`, `after` or `when` is written before the parameter ends. All three
    /// are reserved (`SysML` 8.2.2.1.2), so none can be a name or appear inside a
    /// declaration, and what ends the parameter is reserved too: the `via` of the
    /// receiver, a transition's `if`, `do` or `then`, or a `;`, `{` or `}`.
    fn at_trigger_payload(&self) -> bool {
        let mut n = 0;
        loop {
            if ["at", "after", "when"]
                .iter()
                .any(|word| self.nth_is_keyword(n, word))
            {
                return true;
            }
            if self.peek_nth(n).is_none()
                || ["via", "if", "do", "then"]
                    .iter()
                    .any(|word| self.nth_is_keyword(n, word))
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::LBrace)
                || self.nth_is(n, SyntaxKind::RBrace)
            {
                return false;
            }
            n += 1;
        }
    }

    // production: TriggerValuePart@sysml
    //
    // TriggerValuePart : Feature =
    //     ownedRelationship += TriggerFeatureValue                   (SysML 8.2.2.17.4)
    //
    // production: TriggerFeatureValue@sysml
    //
    // TriggerFeatureValue : FeatureValue =
    //     ownedRelatedElement += TriggerExpression                   (SysML 8.2.2.17.4)
    //
    // production: TriggerExpression@sysml
    //
    // TriggerExpression : TriggerInvocationExpression =
    //       kind = ( 'at' | 'after' ) ownedRelationship += ArgumentMember
    //     | kind = 'when' ownedRelationship += ArgumentExpressionMember
    //                                                                (SysML 8.2.2.17.4)
    //
    // "A change trigger is notated using the keyword when followed by an expression whose
    // result must be a Boolean value"; an absolute time trigger, `at`, and a relative one,
    // `after`, take a TimeInstantValue and a DurationValue (7.17.8, receipt bb0d6dc7).
    // `when` takes its expression as an ArgumentExpressionMember, REFERENCED rather than
    // evaluated once, because a change trigger re-evaluates it; `at` and `after` take an
    // ArgumentMember. The result types are sv2-resolve's to check.
    //
    // The clause prints `kind = ( 'at | 'after' )`, a quote left open; deviations.json
    // records the repair, TriggerExpression, follow_spec: the literals are 'at' and 'after'
    // (SYSML21-401).
    fn trigger_value_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TriggerValuePart);
        self.start_node(SyntaxKind::TriggerFeatureValue);
        self.start_node(SyntaxKind::TriggerExpression);
        if self.at_keyword("when") {
            self.expect_keyword("when");
            self.argument_expression_member(TIER_LOOSEST);
        } else if self.at_keyword("at") {
            self.expect_keyword("at");
            self.argument_member(TIER_LOOSEST);
        } else {
            self.expect_keyword("after");
            self.argument_member(TIER_LOOSEST);
        }
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    /// A state action member, with the entry transitions an entry action takes after it.
    ///
    /// `EntryActionMember EntryTransitionMember* | DoActionMember | ExitActionMember`, three
    /// of `StateBodyItem`'s alternatives (`SysML` 8.2.2.18.1). Only the first has a suffix:
    /// "a succession from the entry action to that state usage, representing that this is
    /// the state that is entered on completion of the entry action" (7.18.2, receipt
    /// 42b13f63), so `do a; then b;` leaves its `then` reported.
    fn state_action_item(&mut self) {
        let n = usize::from(self.at_visibility());
        let (word, node) = if self.nth_is_keyword(n, "entry") {
            ("entry", SyntaxKind::EntryActionMember)
        } else if self.nth_is_keyword(n, "do") {
            ("do", SyntaxKind::DoActionMember)
        } else {
            ("exit", SyntaxKind::ExitActionMember)
        };
        self.state_action_member(word, node);
        if word == "entry" {
            while self.at_entry_transition_member() {
                self.entry_transition_member();
            }
        }
    }

    // production: EntryActionMember@sysml
    //
    // EntryActionMember : StateSubactionMembership =
    //     MemberPrefix kind = 'entry'
    //     ownedRelatedElement += StateActionUsage                    (SysML 8.2.2.18.1)
    //
    // production: DoActionMember@sysml
    //
    // DoActionMember : StateSubactionMembership =
    //     MemberPrefix kind = 'do' ownedRelatedElement += StateActionUsage
    //
    // production: ExitActionMember@sysml
    //
    // ExitActionMember : StateSubactionMembership =
    //     MemberPrefix kind = 'exit' ownedRelatedElement += StateActionUsage
    //
    // One shape, three keywords, each the StateSubactionMembership's kind (8.3.18.4,
    // receipt 75fd3273). Marked although StateActionUsage is not: this production's own
    // parts are read.
    fn state_action_member(&mut self, word: &str, node: SyntaxKind) {
        self.eat_trivia();
        self.start_node(node);
        self.member_prefix();
        self.expect_keyword(word);
        self.state_action_usage();
        self.finish_node();
    }

    // StateActionUsage : ActionUsage =
    //       EmptyActionUsage ';'
    //     | StatePerformActionUsage
    //     | StateAcceptActionUsage
    //     | StateSendActionUsage
    //     | StateAssignmentActionUsage                               (SysML 8.2.2.18.1)
    //
    // production: StateActionUsage@sysml
    //
    // Marked: every alternative is read. No node of its own: the alternative taken is the
    // node.
    //
    // "If the keyword is immediately followed by a semicolon ;, then they are empty
    // actions. If they are followed by a qualified name or feature chain for an action
    // usage, then this is a shorthand for relating the entry, do, or exit action to the
    // identified action usage via reference subsetting" (7.18.2, receipt 42b13f63) —
    // PerformActionUsageDeclaration's first alternative.
    //
    // production: EmptyActionUsage@sysml
    //
    // EmptyActionUsage : ActionUsage = {}                            (SysML 8.2.2.18.1)
    //
    // production: StatePerformActionUsage@sysml
    //
    // StatePerformActionUsage : PerformActionUsage =
    //     PerformActionUsageDeclaration ActionBody                   (SysML 8.2.2.18.1)
    //
    // production: StateAcceptActionUsage@sysml
    //
    // StateAcceptActionUsage : AcceptActionUsage =
    //     AcceptNodeDeclaration ActionBody                           (SysML 8.2.2.18.1)
    //
    // production: StateSendActionUsage@sysml
    //
    // StateSendActionUsage : SendActionUsage =
    //     SendNodeDeclaration ActionBody                             (SysML 8.2.2.18.1)
    //
    // production: StateAssignmentActionUsage@sysml
    //
    // StateAssignmentActionUsage : AssignmentActionUsage =
    //     AssignmentNodeDeclaration ActionBody                       (SysML 8.2.2.18.1)
    //
    // The clause prints the send form's head as `StateSendActionUsage : SendActionUsage`
    // with no `=`, SYSML21-402; deviations.json has both spec_only, follow_spec, since the
    // Pilot reaches the same text through its general action alternatives. "A send action
    // usage must be ... The owned entry, do or exit action of a state definition or usage"
    // (7.17.7, receipt db730711), and 7.17.9 says the same of an assignment (receipt
    // 7d690ecc).
    fn state_action_usage(&mut self) {
        if self.at(SyntaxKind::Semicolon) {
            self.empty_action_usage();
            self.bump();
            return;
        }
        match self.action_node_keyword() {
            Some("accept") => {
                self.eat_trivia();
                self.start_node(SyntaxKind::StateAcceptActionUsage);
                self.accept_node_declaration();
                self.action_body();
                self.finish_node();
            }
            Some("send") => {
                self.eat_trivia();
                self.start_node(SyntaxKind::StateSendActionUsage);
                self.send_node_declaration();
                self.action_body();
                self.finish_node();
            }
            Some(_) => {
                // `assign`, the third of the keywords `action_node_keyword` finds.
                self.eat_trivia();
                self.start_node(SyntaxKind::StateAssignmentActionUsage);
                self.assignment_node_declaration();
                self.action_body();
                self.finish_node();
            }
            None => {
                self.eat_trivia();
                self.start_node(SyntaxKind::StatePerformActionUsage);
                self.perform_action_usage_declaration();
                self.action_body();
                self.finish_node();
            }
        }
    }

    /// Whether an `EntryTransitionMember` starts here.
    ///
    /// `GuardedTargetSuccession` — `if`, an expression, `then` — or `'then'
    /// TransitionSuccession`, each ending in `;` (`SysML` 8.2.2.18.1, with the
    /// `EntryTransitionMember` deviation). A bare `then` is one only when a `ConnectorEnd` and
    /// the `;` follow it: `then state s;` is the next item's `SourceSuccessionMember`.
    fn at_entry_transition_member(&self) -> bool {
        let n = usize::from(self.at_visibility());
        (self.nth_is_keyword(n, "if") && self.scan_for_keyword(n + 1, "then").is_some())
            || (self.nth_is_keyword(n, "then")
                && self
                    .skip_connector_end(n + 1)
                    .is_some_and(|after| self.nth_is(after, SyntaxKind::Semicolon)))
    }

    // production: EntryTransitionMember@sysml
    //
    // EntryTransitionMember : FeatureMembership =
    //     MemberPrefix
    //     ( ownedRelatedElement += GuardedTargetSuccession
    //     | 'then' ownedRelatedElement += TransitionSuccession
    //     ) ';'                                  (SysML 8.2.2.18.1, as the deviation reads it)
    //
    // The specification writes `'then' TargetSuccession`, and TargetSuccession writes its
    // own `then`, so the printed form doubles it. deviations.json records follow_xtext:
    // TransitionSuccession is meant, the corpus writes one `then` (`entry; then S1;`,
    // examples/Simple Tests/StateTest.sysml:13), and no doubled `then` occurs anywhere.
    fn entry_transition_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EntryTransitionMember);
        self.member_prefix();
        if self.at_keyword("if") {
            self.guarded_target_succession();
        } else {
            // deviation: EntryTransitionMember
            self.note_deviation(
                "EntryTransitionMember",
                "`then` and a TransitionSuccession after an entry action",
            );
            self.expect_keyword("then");
            self.transition_succession();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish_node();
    }

    // production: EffectBehaviorMember@sysml
    //
    // EffectBehaviorMember : TransitionFeatureMembership =
    //     'do' { kind = 'effect' }
    //     ownedRelatedElement += EffectBehaviorUsage                 (SysML 8.2.2.18.3)
    //
    // The `do` sets the
    // TransitionFeatureMembership's kind (8.3.18.8, receipt 7818cc5c), as `accept` and
    // `if` set theirs.
    //
    // EffectBehaviorUsage : ActionUsage =
    //       EmptyActionUsage | TransitionPerformActionUsage | TransitionAcceptActionUsage
    //     | TransitionSendActionUsage | TransitionAssignmentActionUsage
    //                                                                (SysML 8.2.2.18.3)
    //
    // production: EffectBehaviorUsage@sysml
    //
    // Marked, as StateActionUsage is: every alternative is read. Its empty form writes no
    // `;`, unlike StateActionUsage's, so `do then b;` is an effect that does nothing.
    //
    // production: TransitionPerformActionUsage@sysml
    //
    // TransitionPerformActionUsage : PerformActionUsage =
    //     PerformActionUsageDeclaration ( '{' ActionBodyItem* '}' )? (SysML 8.2.2.18.3)
    //
    // production: TransitionAcceptActionUsage@sysml
    //
    // TransitionAcceptActionUsage : AcceptActionUsage =
    //     AcceptNodeDeclaration ( '{' ActionBodyItem* '}' )?         (SysML 8.2.2.18.3)
    //
    // production: TransitionSendActionUsage@sysml
    //
    // TransitionSendActionUsage : SendActionUsage =
    //     SendNodeDeclaration ( '{' ActionBodyItem* '}' )?           (SysML 8.2.2.18.3)
    //
    // production: TransitionAssignmentActionUsage@sysml
    //
    // TransitionAssignmentActionUsage : AssignmentActionUsage =
    //     AssignmentNodeDeclaration ( '{' ActionBodyItem* '}' )?     (SysML 8.2.2.18.3)
    //
    // Not ActionBody: the braced form or nothing, never `;`, because the `then` after the
    // effect is what ends it. `do send s to p then S1;` (examples/Simple
    // Tests/StateTest.sysml:36-37) writes it so.
    fn effect_behavior_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EffectBehaviorMember);
        self.expect_keyword("do");
        if self.at_keyword("then") {
            self.empty_action_usage();
        } else {
            match self.action_node_keyword() {
                Some("accept") => {
                    self.eat_trivia();
                    self.start_node(SyntaxKind::TransitionAcceptActionUsage);
                    self.accept_node_declaration();
                    self.optional_action_body_items();
                    self.finish_node();
                }
                Some("send") => {
                    self.eat_trivia();
                    self.start_node(SyntaxKind::TransitionSendActionUsage);
                    self.send_node_declaration();
                    self.optional_action_body_items();
                    self.finish_node();
                }
                Some(_) => {
                    // `assign`, the third of the keywords `action_node_keyword` finds.
                    self.eat_trivia();
                    self.start_node(SyntaxKind::TransitionAssignmentActionUsage);
                    self.assignment_node_declaration();
                    self.optional_action_body_items();
                    self.finish_node();
                }
                None => {
                    self.eat_trivia();
                    self.start_node(SyntaxKind::TransitionPerformActionUsage);
                    self.perform_action_usage_declaration();
                    self.optional_action_body_items();
                    self.finish_node();
                }
            }
        }
        self.finish_node();
    }

    /// Whether an `ExhibitStateUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'exhibit'` (`SysML` 8.2.2.18.2). The keyword names this usage
    /// and nothing else, as `perform` does.
    fn at_exhibit_state_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "exhibit")
    }

    // production: ExhibitStateUsage@sysml
    //
    // ExhibitStateUsage =
    //     OccurrenceUsagePrefix 'exhibit'
    //     ( ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     | 'state' UsageDeclaration )
    //     ValuePart? StateUsageBody                                  (SysML 8.2.2.18.2)
    //
    // "An exhibit state usage is a kind of perform action usage ... for which the action
    // usage is a state usage" (7.18.4, receipt dbedb2cc): PerformActionUsageDeclaration's
    // two alternatives with `state` for `action`, and a state body. The metaclass is
    // ExhibitStateUsage (8.3.18.2, receipt 72f51928). Marked although
    // OccurrenceUsagePrefix is not, as StateUsage is.
    fn exhibit_state_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExhibitStateUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("exhibit");
        if self.at_keyword("state") {
            self.expect_keyword("state");
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.state_body_part(SyntaxKind::StateUsageBody);
        self.finish_node();
    }

    // production: CalculationUsage@sysml
    //
    // CalculationUsage : CalculationUsage =
    //     OccurrenceUsagePrefix 'calc'
    //     ActionUsageDeclaration CalculationBody                    (SysML 8.2.2.19)
    //
    // "A calculation definition or usage is declared as an action definition or usage ...
    // but using the keyword calc instead of action" (7.19.2, receipt 14c3c04e) — so the
    // declaration is ActionUsage's own, read by `action_usage_declaration` under its own
    // name, and only the body differs: a CalculationBody, whose last part may be the
    // result expression. The metaclass is CalculationUsage (8.3.19.3, receipt bd2b9f83),
    // an ActionUsage that is also a KerML Expression.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Calculations::calculations
    // constraint: CalculationUsage::checkCalculationUsageSpecialization,
    //     `specializesFromLibrary('Calculations::calculations')` (8.3.19.3; 8.4.15.2,
    //     receipt d62236d7). An injection, so sv2-hir's; this layer builds the tree only
    //     (ADR-0002).
    // constraint: CalculationUsage::checkCalculationUsageSubcalculationSpecialization
    //     (8.3.19.3): owned by a CalculationDefinition or CalculationUsage, it specializes
    //     `Calculations::Calculation::subcalculations`. sv2-hir's, for the same reason.
    fn calculation_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CalculationUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("calc");
        self.action_usage_declaration();
        self.calculation_body();
        self.finish_node();
    }

    // production: IncludeUseCaseUsage
    //
    // IncludeUseCaseUsage = OccurrenceUsagePrefix 'include'
    //     ( ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     | 'use' 'case' UsageDeclaration )
    //     ValuePart? CaseBody                                    (SysML 8.2.2.25)
    //
    // PerformActionUsageDeclaration's two alternatives over a use case: "declared as a
    // use case usage ... using the kind keyword include use case", or "using just the
    // keyword include ... the included use case ... identified by giving a qualified name
    // or feature chain immediately after the include keyword" (7.25.3, receipt 6f1b9dfd).
    // Told apart on one token, as there: `use` is reserved and a reference opens on a
    // name. The body is a CaseBody because 8.2.2.25 writes one. The metaclass is
    // IncludeUseCaseUsage (8.3.25.2, receipt 40fbe5a7), both a UseCaseUsage and a
    // PerformActionUsage.
    //
    // The reference alternative's FeatureSpecializationPart may open on a multiplicity
    // (`include 'add fuel'[0..*] { }`, training/35. Use Cases/Use Case Usage Example.sysml
    // and validation/18-Use Case), so `[` is asked for as `usage_declaration` asks for it.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: UseCases::UseCase::includedUseCases, when owned by a use case
    // constraint: IncludeUseCaseUsage::checkIncludeUseCaseSpecialization (8.3.25.2). An
    //     injection, so sv2-hir's (ADR-0002).
    // constraint: IncludeUseCaseUsage::validateIncludeUseCaseUsageReference (8.3.25.2):
    //     the reference's target must be a UseCaseUsage. A question of resolution, so
    //     sv2-resolve's.
    fn include_use_case_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::IncludeUseCaseUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("include");
        if self.at_keyword("use") {
            self.bump_as(keyword("use").unwrap_or(SyntaxKind::BasicName));
            self.expect_keyword("case");
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.case_body();
        self.finish_node();
    }

    // production: ConstraintDefinition
    //
    // ConstraintDefinition = OccurrenceDefinitionPrefix 'constraint' 'def'
    //                        DefinitionDeclaration CalculationBody   (SysML 8.2.2.20)
    //
    // Off the SIMPLE_DEFINITIONS spine for the reason RequirementDefinition is: it names
    // the declaration and the body separately rather than taking a Definition, so there
    // is no Definition node in its tree.
    //
    // Was here as the ONLY caller that made CalculationBody reachable — a body
    // production with no caller cannot be tested, and an untested production is a claim.
    // CalculationDefinition (8.2.2.19) is now the second, which is the clause that names
    // the body in the first place.
    fn constraint_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstraintDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("constraint");
        self.expect_keyword("def");
        self.definition_declaration();
        self.calculation_body();
        self.finish_node();
    }

    // production: TransitionSuccessionMember@sysml
    //
    // TransitionSuccessionMember : OwningMembership =
    //     ownedRelatedElement += TransitionSuccession               (SysML 8.2.2.18.3)
    //
    // production: TransitionSuccession@sysml
    //
    // TransitionSuccession : Succession =
    //     ownedRelationship += EmptyEndMember
    //     ownedRelationship += ConnectorEndMember                   (SysML 8.2.2.18.3)
    //
    // production: EmptyEndMember@sysml
    //
    // EmptyEndMember : EndFeatureMembership =
    //     ownedRelatedElement += EmptyFeature                       (SysML 8.2.2.18.3)
    //
    // A Succession, not a SuccessionAsUsage, so its source end carries nothing at all —
    // an EmptyFeature written nowhere in the text (receipt 2da333dc), as
    // EmptyResultMember's is (KerML 8.2.5.8.1, receipt 423205c7: a different clause and a
    // different receipt, which is why each is named beside the claim it carries).
    // TargetSuccession's SourceEnd differs: it may carry an OwnedMultiplicity.
    // Trivia is not eaten before the empty end, or it would land inside a node the
    // author never wrote.
    fn transition_succession_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TransitionSuccessionMember);
        self.transition_succession();
        self.finish_node();
    }

    /// The `TransitionSuccession` alone, which `EntryTransitionMember` owns with no
    /// `TransitionSuccessionMember` around it (the `EntryTransitionMember` deviation).
    fn transition_succession(&mut self) {
        self.start_node(SyntaxKind::TransitionSuccession);
        self.start_node(SyntaxKind::EmptyEndMember);
        self.start_node(SyntaxKind::EmptyFeature);
        self.finish_node();
        self.finish_node();
        self.connector_end_member();
        self.finish_node();
    }

    // production: CalculationDefinition
    //
    // CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
    //                         DefinitionDeclaration CalculationBody  (SysML 8.2.2.19)
    //
    // ConstraintDefinition (8.2.2.20) differing in ONE KEYWORD, over the body the two
    // share, and off the SIMPLE_DEFINITIONS spine for the same reason: it names the
    // declaration and the body separately rather than taking a Definition.
    //
    // The metaclass is SysML::CalculationDefinition (8.3.19.2), an ActionDefinition that
    // is also a Function. BOTH it and the sibling it shares a body with are
    // OccurrenceDefinitions, and the chains say how:
    //
    //     CalculationDefinition > ActionDefinition > Behavior > OccurrenceDefinition
    //     ConstraintDefinition  > Predicate                   > OccurrenceDefinition
    //
    // so what separates them is the ROUTE and the Function, not the presence of
    // OccurrenceDefinition. An earlier revision of this comment said CalculationDefinition
    // was NOT an OccurrenceDefinition, which is false and was caught in review; it is why
    // the chain is written out here rather than summarised.
    //
    // implied specialization: Calculations::Calculation
    // constraint: CalculationDefinition::checkCalculationDefinitionSpecialization
    //     `specializesFromLibrary('Calculations::Calculation')` (SysML 8.3.19.2). It
    //     also inherits checkActionDefinitionSpecialization (8.3.17.3,
    //     `Actions::Action`) and KerML's checkBehaviorSpecialization (8.3.4.6.2,
    //     `Performances::Performance`). All three are injections, so they belong in
    //     sv2-hir, which does not exist yet; this layer builds the tree only (ADR-0002).
    //
    // All thirteen .sysml files that write `calc def` also write `return`, a
    // ReturnParameterMember (8.2.2.19), read in a calculation body by `body_specific_item`.
    fn calculation_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CalculationDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("calc");
        self.expect_keyword("def");
        self.definition_declaration();
        self.calculation_body();
        self.finish_node();
    }

    // production: CalculationBody
    //
    // CalculationBody : Type = ';' | '{' CalculationBodyPart '}'    (SysML 8.2.2.19)
    fn calculation_body(&mut self) {
        self.eat_trivia();
        if self.at(SyntaxKind::Semicolon) {
            self.start_node(SyntaxKind::CalculationBody);
            self.bump();
            self.finish_node();
        } else if self.at(SyntaxKind::LBrace) {
            self.braced_calculation_body();
        } else {
            self.start_node(SyntaxKind::CalculationBody);
            self.error_expected("`;` or `{` after a calculation or constraint declaration");
            self.finish_node();
        }
    }

    /// `CalculationBody`'s second alternative alone, `'{' CalculationBodyPart '}'`: what
    /// `SysML`'s `ExpressionBody` reads under its narrowing (see `body_expression`). The
    /// `;` is not offered here, so the narrowing holds whatever the caller dispatched on.
    fn braced_calculation_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CalculationBody);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.depth += 1;
        self.calculation_body_part();
        self.depth -= 1;
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish_node();
    }

    // production: CalculationBodyPart@sysml
    //
    // CalculationBodyPart : Type =
    //     CalculationBodyItem* ( ownedRelationship += ResultExpressionMember )?
    //                                                            (SysML 8.2.2.19)
    //
    // CalculationBodyItem = ActionBodyItem | ReturnParameterMember is read by
    // `body_elements` under Body::Calculation, and marked at `body_element`; the
    // ResultExpressionMember after the run is read here.
    //
    // The star is greedy and the expression is last; `at_result_expression` is where
    // that boundary is decided, and it is the whole of the difficulty here.
    fn calculation_body_part(&mut self) {
        // A leading `/* ... */` is the run's first member, so it is left for the loop.
        self.with_significant_comments(Self::eat_trivia);
        self.start_node(SyntaxKind::CalculationBodyPart);
        self.body_elements(Some(SyntaxKind::RBrace), Body::Calculation);
        // `body_elements` returns either at the `}` or because the item run ended. The
        // second is the only case with an expression to read, and the `?` in the
        // production is exactly this test.
        if !self.at_end() && !self.at(SyntaxKind::RBrace) {
            self.result_expression_member();
        }
        self.finish_node();
    }

    // production: ResultExpressionMember@sysml
    //
    // ResultExpressionMember : ResultExpressionMembership =
    //     MemberPrefix? ownedRelatedElement += OwnedExpression   (SysML 8.2.2.19)
    //
    // production: ResultExpressionMember@kerml
    //
    // ResultExpressionMember : ResultExpressionMembership =
    //     MemberPrefix ownedRelatedElement += OwnedExpression    (KerML 8.2.5.7.1)
    //
    // One method, two markers: KerML writes no `?`, and MemberPrefix derives the empty
    // string either way, so the text is the same. KerML reaches it from FunctionBodyPart.
    //
    // The metaclass is KerML's ResultExpressionMembership (8.3.4.7.7), a
    // FeatureMembership. validateResultExpressionMembershipOwningType says its owningType
    // must be a Function or an Expression; that is a constraint and not this layer's
    // (ADR-0002), and the grammar already reaches this production only from a
    // calculation body.
    //
    // The `?` on MemberPrefix is redundant — MemberPrefix is itself `VisibilityIndicator?`
    // and already derives the empty string. The decision to keep the clause as written
    // rather than drop it as SysML.xtext does is recorded in the DERIVED UNIT's notes,
    // .claude/state/grammar/units/ResultExpressionMember@sysml.json, and NOT in
    // deviations.json, which has no entry for this production. The node is built either
    // way, as MemberPrefix's always is, so the redundancy costs nothing here.
    //
    // No terminating semicolon. That is the whole reason this member is told from an
    // item by lookahead rather than by its first token.
    fn result_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ResultExpressionMember);
        self.member_prefix();
        self.owned_expression();
        self.finish_node();
    }

    // production: RequirementConstraintMember
    //
    // RequirementConstraintMember : RequirementConstraintMembership =
    //     MemberPrefix? RequirementKind
    //     ownedRelatedElement += RequirementConstraintUsage      (SysML 8.2.2.21.1)
    //
    // production: RequirementKind
    //
    // RequirementKind = 'assume' { kind = 'assumption' }
    //                 | 'require' { kind = 'requirement' }       (SysML 8.2.2.21.1)
    //
    // The metaclass is SysML::RequirementConstraintMembership (8.3.21.7), a
    // FeatureMembership carrying a `kind` that says which of the two keywords was
    // written. RequirementKind is the production that sets it, and it gets a node of its
    // own for the reason PortionKind and VisibilityIndicator do: the keyword is the only
    // record of an attribute that has no other spelling in the text.
    //
    // MemberPrefix? — the `?` is redundant, as it is on ResultExpressionMember:
    // MemberPrefix is itself `VisibilityIndicator?` and already derives the empty string.
    // Kept because the clause writes it; see that production's derived unit for the
    // reasoning. The node is built either way.
    fn requirement_constraint_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementConstraintMember);
        self.member_prefix();
        self.requirement_kind();
        self.requirement_constraint_usage();
        self.finish_node();
    }

    fn requirement_kind(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementKind);
        match ["assume", "require"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`assume` or `require`"),
        }
        self.finish_node();
    }

    // production: RequirementConstraintUsage@sysml
    //
    // RequirementConstraintUsage : ConstraintUsage =
    //     ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     RequirementBody
    //   | ( UsageExtensionKeyword* 'constraint' | UsageExtensionKeyword+ )
    //     ConstraintUsageDeclaration CalculationBody              (SysML 8.2.2.21.1)
    //
    // The second alternative's `UsageExtensionKeyword+` is prefix metadata standing in
    // for the `constraint` keyword entirely, so `require #approved { a <= b }` is that
    // alternative with no `constraint`: `( X* 'constraint' | X+ )` is "a `#` or a
    // `constraint`", then the keywords, then the keyword if written.
    //
    // THE TWO ALTERNATIVES TAKE DIFFERENT BODIES, and that is the adjudicated conflict.
    // The clause gives the by-reference alternative a RequirementBody and the Pilot gives
    // it a CalculationBody; deviations.json records follow_spec, adjudicated 2026-09-17,
    // so a referenced constraint takes a RequirementBody and only the `constraint` form
    // ends in an expression. The difference is real rather than cosmetic: a
    // RequirementBody admits a subject and a nested requirement, a CalculationBody admits
    // a trailing ResultExpressionMember, and no body admits both.
    //
    // The alternatives are told apart BEFORE either body begins, which is what makes the
    // conflict harmless to read: the second opens on the keyword `constraint` or a `#`
    // and the first on a QualifiedName, and a keyword is not a name (SysML 8.2.2.1.2).
    fn requirement_constraint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementConstraintUsage);
        if self.at_keyword("constraint") || self.at(SyntaxKind::Hash) {
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.eat_optional_keyword("constraint");
            self.constraint_usage_declaration();
            self.calculation_body();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
            self.requirement_body();
        }
        self.finish_node();
    }

    // production: ConstraintUsageDeclaration
    //
    // ConstraintUsageDeclaration : ConstraintUsage =
    //     UsageDeclaration ValuePart?                            (SysML 8.2.2.20)
    //
    // Shared by four productions: RequirementConstraintUsage behind `require` or
    // `assume`, AssertConstraintUsage behind `assert constraint`, ConstraintUsage, the bare
    // `constraint c { }` at member position, and RequirementUsage.
    fn constraint_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstraintUsageDeclaration);
        self.usage_declaration();
        if self.at_value_part() {
            self.value_part();
        }
        self.finish_node();
    }

    /// Whether a `RequirementUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'requirement'` with no `def` after it (`SysML` 8.2.2.21.2):
    /// the `def` is what makes it the `RequirementDefinition` beside it. The prefix skipped
    /// is the one `requirement_usage` reads with `occurrence_usage_prefix`.
    fn at_requirement_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "requirement") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: RequirementUsage@sysml
    //
    // RequirementUsage =
    //     OccurrenceUsagePrefix 'requirement'
    //     ConstraintUsageDeclaration RequirementBody               (SysML 8.2.2.21.2)
    //
    // "A requirement definition or usage is declared as a kind of constraint definition
    // or usage ... using the kind keyword requirement" (7.21.2, receipt 021b9219): hence
    // ConstraintUsageDeclaration, read by `constraint_usage_declaration`, and the
    // RequirementBody that RequirementDefinition reads, which ends in no result
    // expression. "If a requirement definition or usage is declared with a short name ...
    // then this is also considered to be its requirement ID" -- the `<'1.1'>` the corpus
    // writes, read by the declaration's Identification. The metaclass is RequirementUsage
    // (8.3.21.9, receipt 8a719041), a ConstraintUsage.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Requirements::requirementChecks
    // constraint: RequirementUsage::checkRequirementUsageSpecialization,
    //     `specializesFromLibrary('Requirements::requirementChecks')` (8.3.21.9; 8.4.17.2,
    //     receipt da666b19). An injection, so sv2-hir's; this layer builds the tree only
    //     (ADR-0002).
    // constraint: RequirementUsage::checkRequirementUsageSubrequirementSpecialization
    //     (8.3.21.9): composite, and owned by a RequirementDefinition or RequirementUsage,
    //     it specializes `Requirements::RequirementCheck::subrequirements`. sv2-hir's too.
    fn requirement_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("requirement");
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ConcernUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'concern'` with no `def` after it (`SysML` 8.2.2.21.3), as
    /// `at_requirement_usage` asks of `requirement`.
    fn at_concern_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "concern") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ConcernUsage@sysml
    //
    // ConcernUsage =
    //     OccurrenceUsagePrefix 'concern'
    //     ConstraintUsageDeclaration RequirementBody             (SysML 8.2.2.21.3)
    //
    // RequirementUsage's shape with the kind keyword `concern` (7.21.3, receipt 0e50a373);
    // deviation ConcernUsageKeyword (xtext_only, follow_spec) matches the literal. The
    // metaclass is ConcernUsage (8.3.21.4, receipt 7b9ce89b), a RequirementUsage. A
    // BehaviorUsageElement (8.2.2.6.4), as RequirementUsage is.
    //
    // implied specialization: Requirements::concernChecks, and
    //     Requirements::RequirementCheck::concerns when framed
    // constraint: ConcernUsage::checkConcernUsageSpecialization and
    //     checkConcernUsageFramedConcernSpecialization (8.3.21.4). Injections, so sv2-hir's
    //     (ADR-0002).
    fn concern_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConcernUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("concern");
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ViewpointUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'viewpoint'` with no `def` after it (`SysML` 8.2.2.26.3), as
    /// `at_concern_usage` asks of `concern`.
    fn at_viewpoint_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "viewpoint") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewpointUsage@sysml
    //
    // ViewpointUsage =
    //     OccurrenceUsagePrefix 'viewpoint'
    //     ConstraintUsageDeclaration RequirementBody             (SysML 8.2.2.26.3)
    //
    // RequirementUsage's shape with the kind keyword `viewpoint` (7.26.3, receipt
    // 1813f74a); deviation ViewpointUsageKeyword (xtext_only, follow_spec) matches the
    // literal. The metaclass is ViewpointUsage (8.3.26.9, receipt 26897969), a
    // RequirementUsage. A BehaviorUsageElement (8.2.2.6.4), as ConcernUsage is.
    //
    // implied specialization: Views::viewpoints, and Views::View::viewpointSatisfactions
    //     when composite and owned by a view
    // constraint: ViewpointUsage::checkViewpointUsageSpecialization and
    //     checkViewpointUsageViewpointSatisfactionSpecialization (8.3.26.9). Injections, so
    //     sv2-hir's (ADR-0002).
    fn viewpoint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewpointUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("viewpoint");
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ConstraintUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'constraint'` with no `def` after it (`SysML` 8.2.2.20): the
    /// `def` is what makes it the `ConstraintDefinition` beside it, as for `calc`. An
    /// `assert` or `require` before `constraint` is a different production, and neither is
    /// in the prefix skipped, so neither is claimed here. The prefix skipped is the one
    /// `constraint_usage` reads with `occurrence_usage_prefix`.
    fn at_constraint_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "constraint") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ConstraintUsage@sysml
    //
    // ConstraintUsage =
    //     OccurrenceUsagePrefix 'constraint'
    //     ConstraintUsageDeclaration CalculationBody                (SysML 8.2.2.20)
    //
    // "A constraint definition or usage can be declared as a kind of occurrence
    // definition or usage ... using the kind keyword constraint", and its body "is also
    // like the body of a calculation definition or usage ... including the addition of
    // the declaration of a result expression at the end" (7.20.2, receipt 0014441c). The
    // metaclass is ConstraintUsage (8.3.20.4, receipt 81ca78cd), an OccurrenceUsage that
    // is also a KerML BooleanExpression.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Constraints::constraintChecks
    // constraint: ConstraintUsage::checkConstraintUsageSpecialization,
    //     `specializesFromLibrary('Constraints::constraintChecks')` (8.3.20.4; 8.4.16.2,
    //     receipt d7eb8ca4). An injection, so sv2-hir's; this layer builds the tree only
    //     (ADR-0002).
    // constraint: ConstraintUsage::checkConstraintUsageCheckedConstraintSpecialization
    //     (8.3.20.4): owned by an ItemDefinition or ItemUsage, it specializes
    //     `Items::Item::checkedConstraints`. sv2-hir's, for the same reason.
    fn constraint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstraintUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("constraint");
        self.constraint_usage_declaration();
        self.calculation_body();
        self.finish_node();
    }

    /// Whether an `AssertConstraintUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix`, then `assert`. The keyword is reserved (`SysML` 8.2.2.1.2)
    /// and opens one other production, `SatisfyRequirementUsage` (8.2.2.21.2), whose
    /// `satisfy` comes after the same optional `not`: that is declined here and read by
    /// `satisfy_requirement_usage`, rather than read as an assertion referencing `satisfy`,
    /// which is reserved and cannot be a name anyway.
    ///
    /// The prefix skipped is `OccurrenceUsagePrefix`, and `assert_constraint_usage` reads
    /// exactly that with `occurrence_usage_prefix` — the pairing that must match, since a
    /// recogniser that looks past more than its production reads leaves the body loop at
    /// the same token.
    fn at_assert_constraint_usage(&self, n: usize) -> bool {
        let n = self.skip_occurrence_usage_prefix(n);
        if !self.nth_is_keyword(n, "assert") {
            return false;
        }
        let after = n + 1 + usize::from(self.nth_is_keyword(n + 1, "not"));
        !self.nth_is_keyword(after, "satisfy")
    }

    // production: AssertConstraintUsage@sysml
    //
    // AssertConstraintUsage =
    //     OccurrenceUsagePrefix 'assert' ( isNegated ?= 'not' )?
    //     ( ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart?
    //     | 'constraint' ConstraintUsageDeclaration )
    //     CalculationBody                                          (SysML 8.2.2.20)
    //
    // The metaclass is AssertConstraintUsage (8.3.20.2, receipt 11f7047a), a
    // ConstraintUsage that is also a KerML Invariant. "An assert constraint usage is
    // declared like a regular constraint usage ... except using the kind keyword assert
    // constraint", and "may also be declared using just the keyword assert", naming the
    // constraint asserted "immediately after the assert keyword" (7.20.3, receipt
    // 44d633db). The alternatives are told apart on their first token: `constraint` is
    // reserved, and the other opens on a QualifiedName.
    //
    // Marked although OccurrenceUsagePrefix is not: this production's own parts are all
    // read, and OccurrenceUsagePrefix's two gaps (EndUsagePrefix, UsageExtensionKeyword)
    // are every occurrence usage's, as they are ActionUsage's.
    //
    // implied specialization: Constraints::assertedConstraintChecks, or
    //     Constraints::negatedConstraintChecks when `not` is written
    // constraint: AssertConstraintUsage::checkAssertConstraintUsageSpecialization,
    //     `if isNegated then specializesFromLibrary('Constraints::negatedConstraintChecks')
    //     else specializesFromLibrary('Constraints::assertedConstraintChecks') endif`
    //     (8.3.20.2; 8.4.16.3, receipt 9a163399). An injection, so sv2-hir's; this layer
    //     builds the tree only (ADR-0002).
    // constraint: AssertConstraintUsage::validateAssertConstraintUsageReference
    //     (8.3.20.2): the reference alternative's target must be a ConstraintUsage. A
    //     question of resolution, so sv2-resolve's.
    fn assert_constraint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AssertConstraintUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("assert");
        self.eat_optional_keyword("not");
        if self.at_keyword("constraint") {
            self.bump_as(keyword("constraint").unwrap_or(SyntaxKind::BasicName));
            self.constraint_usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        self.calculation_body();
        self.finish_node();
    }

    /// Whether a `SatisfyRequirementUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'assert'? 'not'? 'satisfy'` (`SysML` 8.2.2.21.2, with the two
    /// `?` of deviation `SatisfyRequirementUsage`). `satisfy` is reserved and opens nothing
    /// else, so it decides, and `at_assert_constraint_usage` declines exactly what this
    /// accepts after an `assert`.
    fn at_satisfy_requirement_usage(&self, n: usize) -> bool {
        let n = self.skip_occurrence_usage_prefix(n);
        let n = n + usize::from(self.nth_is_keyword(n, "assert"));
        let n = n + usize::from(self.nth_is_keyword(n, "not"));
        self.nth_is_keyword(n, "satisfy")
    }

    // production: SatisfyRequirementUsage@sysml
    //
    // SatisfyRequirementUsage =
    //     OccurrenceUsagePrefix 'assert' ( isNegated ?= 'not' ) 'satisfy'
    //     ( ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart?
    //     | 'requirement' UsageDeclaration )
    //     ValuePart?
    //     ( 'by' ownedRelationship += SatisfactionSubjectMember )?
    //     RequirementBody                                        (SysML 8.2.2.21.2)
    //
    // "A satisfy requirement usage is declared as a requirement usage (see 7.21.2 ), using
    // the kind keyword satisfy requirement ... A satisfy requirement usage may also be
    // declared using just the keyword satisfy ... the requirement to be satisfied is
    // identified by giving a qualified name or feature chain immediately after the satisfy
    // keyword" (7.21.4, receipt 05678331). PerformActionUsageDeclaration's two
    // alternatives again, told apart on one token: `requirement` is reserved and a
    // reference opens on a name. The metaclass is SatisfyRequirementUsage (8.3.21.10,
    // receipt e9c53cb3), both an AssertConstraintUsage and a RequirementUsage.
    //
    // The clause writes `'assert'` and `( isNegated ?= 'not' )` with no `?` on either, so
    // the only spelling it admits is `assert not satisfy`. Deviation SatisfyRequirementUsage
    // (follow_xtext) makes each optional, on 7.21.4's own `satisfy vehicleMaximumMass by
    // vehicle1;` and `not satisfy ...` and the corpus's use of all four; the order is kept.
    //
    // The reference alternative's FeatureSpecializationPart may open on a multiplicity, as
    // `event`'s and `include`'s do, so `[` is asked for.
    //
    // implied specialization: Requirements::satisfiedRequirementChecks, or
    //     ::notSatisfiedRequirementChecks when negated
    // constraint: SatisfyRequirementUsage::checkSatisfyRequirementUsageSpecialization and
    //     checkSatisfyRequirementUsageBindingConnector (8.3.21.10): the specialization, and
    //     the binding of the subject parameter to the satisfying feature. Injections, so
    //     sv2-hir's (ADR-0002).
    // constraint: SatisfyRequirementUsage::validateSatisfyRequirementUsageReference
    //     (8.3.21.10): the reference's target is a RequirementUsage. sv2-resolve's.
    fn satisfy_requirement_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SatisfyRequirementUsage);
        self.occurrence_usage_prefix();
        if !(self.at_keyword("assert") && self.nth_is_keyword(1, "not")) {
            // deviation: SatisfyRequirementUsage
            self.note_deviation(
                "SatisfyRequirementUsage",
                "a satisfy without both `assert` and `not`",
            );
        }
        self.eat_optional_keyword("assert");
        self.eat_optional_keyword("not");
        self.expect_keyword("satisfy");
        if self.at_keyword("requirement") {
            self.bump_as(keyword("requirement").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        if self.at_value_part() {
            self.value_part();
        }
        if self.at_keyword("by") {
            self.bump_as(keyword("by").unwrap_or(SyntaxKind::BasicName));
            self.satisfaction_subject_member();
        }
        self.requirement_body();
        self.finish_node();
    }

    // production: SatisfactionSubjectMember@sysml
    //
    // SatisfactionSubjectMember : SubjectMembership =
    //     ownedRelatedElement += SatisfactionParameter
    //
    // production: SatisfactionParameter@sysml
    //
    // SatisfactionParameter : ReferenceUsage =
    //     ownedRelationship += SatisfactionFeatureValue
    //
    // production: SatisfactionFeatureValue@sysml
    //
    // SatisfactionFeatureValue : FeatureValue =
    //     ownedRelatedElement += SatisfactionReferenceExpression
    //
    // production: SatisfactionReferenceExpression@sysml
    //
    // SatisfactionReferenceExpression : FeatureReferenceExpression =
    //     ownedRelationship += FeatureChainMember                (SysML 8.2.2.21.2)
    //
    // "The satisfying feature for a satisfy requirement usage can be specified in its
    // declaration, immediately before its body, after keyword by" (7.21.4). Four elements
    // over one reference: the subject parameter, whose value is an expression that
    // references the satisfying feature. Each is a node, as the productions write each; the
    // text is a FeatureChainMember, SysML's (8.2.2.17.5), a name or a chain and nothing
    // more, so `by f(x)` is reported.
    fn satisfaction_subject_member(&mut self) {
        for node in [
            SyntaxKind::SatisfactionSubjectMember,
            SyntaxKind::SatisfactionParameter,
            SyntaxKind::SatisfactionFeatureValue,
            SyntaxKind::SatisfactionReferenceExpression,
        ] {
            self.eat_trivia();
            self.start_node(node);
        }
        self.sysml_feature_chain_member();
        for _ in 0..4 {
            self.finish_node();
        }
    }

    // production: SubjectMember
    //
    // SubjectMember : SubjectMembership =
    //     MemberPrefix ownedRelatedElement += SubjectUsage      (SysML 8.2.2.21.1)
    //
    // The metaclass is SysML::SubjectMembership (8.3.21.11), a ParameterMembership.
    // validateSubjectMembershipOwningType says its owningType must be a
    // RequirementDefinition, RequirementUsage, CaseDefinition or CaseUsage. That is a
    // constraint and not this layer's to enforce (ADR-0002: validity gates writes, never
    // reads) — but the grammar already says the same thing structurally, because
    // SubjectMember is reachable only from RequirementBodyItem and CaseBodyItem. A
    // `subject` in a part definition body is not a rejected SubjectMember; it is not a
    // SubjectMember at all, which is what `Body::admits_subject` decides.
    fn subject_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SubjectMember);
        self.member_prefix();
        self.subject_usage();
        self.finish_node();
    }

    // production: ActorMember
    //
    // ActorMember : ActorMembership =
    //     MemberPrefix ownedRelatedElement += ActorUsage         (SysML 8.2.2.21.1)
    //
    // SubjectMember's sibling, in the same two bodies. "A requirement definition or usage
    // may also have one or more actor or stakeholder parameters ... declared using the
    // keywords actor and stakeholder rather than explicitly declaring their direction"
    // (7.21.2, receipt 021b9219), and a case likewise (7.22.2, receipt eb25a69f). The
    // metaclass is ActorMembership (8.3.21.2, receipt e2ea19a6), a ParameterMembership.
    //
    // constraint: ActorMembership::validateActorMembershipOwningType (8.3.21.2): a
    //     requirement or case owner. The grammar does NOT guarantee it: a referenced
    //     requirement constraint (`require c { actor a; }`) takes a RequirementBody
    //     (8.2.2.21.1), whose owner is a ConstraintUsage. sv2-resolve must check it.
    fn actor_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActorMember);
        self.member_prefix();
        self.actor_usage();
        self.finish_node();
    }

    // production: StakeholderMember@sysml
    //
    // StakeholderMember : StakeholderMembership =
    //     MemberPrefix ownedRelatedElement += StakeholderUsage   (SysML 8.2.2.21.1)
    //
    // production: StakeholderUsage@sysml
    //
    // StakeholderUsage : PartUsage =
    //     'stakeholder' UsageExtensionKeyword* Usage             (SysML 8.2.2.21.1)
    //
    // ActorMember's shape over `stakeholder`: "A requirement definition or usage may also
    // have one or more actor or stakeholder parameters ... declared using the keywords
    // actor and stakeholder rather than explicitly declaring their direction" (7.21.2,
    // receipt 021b9219). The metaclass is StakeholderMembership (8.3.21.12, receipt
    // 9d209633) over a PartUsage.
    //
    // implied specialization: Requirements::RequirementCheck::stakeholders
    // constraint: PartUsage::checkPartUsageStakeholderSpecialization (8.3.11.3), as
    //     `actor_usage` cites checkPartUsageActorSpecialization. An injection that depends
    //     on the owning membership, so sv2-hir's (ADR-0002).
    // constraint: StakeholderMembership::validateStakeholderMembershipOwningType
    //     (8.3.21.12): the owner is a requirement definition or usage. `admits_stakeholder`
    //     gives the grammar's part of that; RequirementConstraintUsage's reference
    //     alternative takes a RequirementBody too, so `require c { stakeholder s; }`
    //     parses, and the constraint is sv2-resolve's to raise, as for an actor.
    fn stakeholder_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::StakeholderMember);
        self.member_prefix();
        self.eat_trivia();
        self.start_node(SyntaxKind::StakeholderUsage);
        self.expect_keyword("stakeholder");
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.usage();
        self.finish_node();
        self.finish_node();
    }

    // production: FramedConcernMember@sysml
    //
    // FramedConcernMember : FramedConcernMembership =
    //     MemberPrefix? 'frame'
    //     ownedRelatedElement += FramedConcernUsage              (SysML 8.2.2.21.1)
    //
    // "A framed concern usage is a subrequirement usage (see 7.21.2 ) indicated by
    // prefixing a concern usage declaration with the keyword frame. As for an assumed or
    // required constraint, the keyword frame can be used rather than frame concern to
    // declare a framed concern using reference subsetting" (7.21.3, receipt 0e50a373).
    // RequirementConstraintMember's shape with the one kind `frame`; the Pilot's
    // FramedConcernKind is deviation FramedConcernKind (xtext_only, follow_spec). The
    // metaclass is FramedConcernMembership (8.3.21.5, receipt 6d58b568).
    //
    // constraint: FramedConcernMembership::validateFramedConcernMembershipConstraintKind
    //     (8.3.21.5): `kind = requirement`, which `frame` sets and no text can change.
    fn framed_concern_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FramedConcernMember);
        self.member_prefix();
        self.expect_keyword("frame");
        self.framed_concern_usage();
        self.finish_node();
    }

    // production: FramedConcernUsage@sysml
    //
    // FramedConcernUsage : ConcernUsage =
    //       ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart? RequirementBody
    //     | ( UsageExtensionKeyword* 'concern'
    //       | UsageExtensionKeyword+ )
    //       ConstraintUsageDeclaration RequirementBody          (SysML 8.2.2.21.1, as
    //                                        deviation FramedConcernUsage reads it)
    //
    // The printed clause gives both alternatives a CalculationBody and the second the
    // undefined CalculationUsageDeclaration (SYSML21-366); deviation FramedConcernUsage
    // (follow_xtext) reads both as RequirementUsage reads its own. What that admits beyond
    // the print, and so where its note falls:
    //
    // - the second alternative, whole: the printed one names a production nothing
    //   defines, so no text reaches it. Noted once, at its first token.
    // - in the first, the body's items that a CalculationBody could not hold.
    //   CalculationBodyItem has every DefinitionBodyItem, through ActionBodyItem
    //   (8.2.2.17.1, 8.2.2.19), and none of RequirementBodyItem's six own members, so
    //   those six are the difference: `framed_concern_body` marks the body's depth and
    //   `note_framed_concern_body_item` notes each of them read there.
    //
    // It also narrows: every CalculationBodyItem that is not a DefinitionBodyItem is no
    // RequirementBodyItem either -- the result expression (`frame c { x }`), a
    // ReturnParameterMember, and ActionBodyItem's control-flow items: InitialNodeMember,
    // an ActionNodeMember, and the target and guarded successions (8.2.2.17.1, 8.2.2.19)
    // -- so the deviation rejects what the print admits. Rejected text carries no note,
    // and tests say so.
    //
    // Told apart as RequirementConstraintUsage's alternatives are: `concern` is reserved and
    // `#` opens no name. The reference alternative's FeatureSpecializationPart may open on
    // a multiplicity (`frame c3[0..*];`, examples/Simple Tests/RequirementTest.sysml:36), so
    // `[` is asked for.
    fn framed_concern_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FramedConcernUsage);
        let saved = self.framed_concern_body.take();
        if self.at_keyword("concern") || self.at(SyntaxKind::Hash) {
            // deviation: FramedConcernUsage
            self.note_deviation(
                "FramedConcernUsage",
                "a framed concern declared with `concern` or a user-defined keyword",
            );
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.eat_optional_keyword("concern");
            self.constraint_usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
            self.framed_concern_body = Some(self.depth + 1);
        }
        self.requirement_body();
        self.framed_concern_body = saved;
        self.finish_node();
    }

    /// A `PARSE-DEVIATION` note for one of `RequirementBodyItem`'s six own members, read
    /// directly in a framed concern's reference-alternative body: text that the printed
    /// `CalculationBody` could not hold and deviation `FramedConcernUsage` admits. See
    /// `framed_concern_usage`. Nothing anywhere else.
    fn note_framed_concern_body_item(&mut self, body: Body) {
        if body == Body::Requirement && self.framed_concern_body == Some(self.depth) {
            // deviation: FramedConcernUsage
            self.note_deviation(
                "FramedConcernUsage",
                "a requirement member in a framed concern's body",
            );
        }
    }

    // production: ActorUsage@sysml
    //
    // ActorUsage : PartUsage =
    //     'actor' UsageExtensionKeyword* Usage                  (SysML 8.2.2.21.1)
    //
    // The keywords come AFTER `actor` here, where a prefix writes them before its kind
    // keyword, so `#m actor a;` is reported and `actor #m a;` read. The metaclass is
    // PartUsage: "Actor and stakeholder parameters are part usages, so they must be
    // (explicitly or implicitly) defined by part definitions" (7.21.2, receipt 021b9219).
    //
    // implied specialization: Requirements::RequirementCheck::actors in a requirement,
    //     Cases::Case::actors otherwise
    // constraint: PartUsage::checkPartUsageActorSpecialization (8.3.11.3). An injection
    //     that depends on the owner, so sv2-hir's (ADR-0002).
    fn actor_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActorUsage);
        self.expect_keyword("actor");
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.usage();
        self.finish_node();
    }

    // production: RequirementVerificationMember
    //
    // RequirementVerificationMember : RequirementVerificationMembership =
    //     MemberPrefix 'verify' { kind = 'requirement' }
    //     ownedRelatedElement += RequirementVerificationUsage    (SysML 8.2.2.24)
    //
    // "A requirement verification usage is a subrequirement of the objective that is
    // indicated by prefixing a requirement usage declaration with the keyword verify. As
    // for an assumed or required constraint, the keyword verify can be used rather than
    // verify requirement to declare a verified requirement using reference subsetting"
    // (7.24.2, receipt d518fc8c). The metaclass is RequirementVerificationMembership
    // (8.3.24.2, receipt 213c087b), a RequirementConstraintMembership.
    //
    // The `kind = 'requirement'` is set by the keyword itself, so unlike
    // RequirementConstraintMember there is no RequirementKind to read: `verify` is the
    // only spelling. validateRequirementVerificationMembershipKind (8.3.24.2) says the
    // same of the metaclass.
    //
    // constraint: RequirementVerificationMembership::
    //     validateRequirementVerificationMembershipOwningType (8.3.24.2): the owner is a
    //     RequirementUsage owned through an ObjectiveMembership. The grammar reaches this
    //     from every RequirementBody, so it does not guarantee it; sv2-resolve's.
    fn requirement_verification_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementVerificationMember);
        self.member_prefix();
        self.expect_keyword("verify");
        self.requirement_verification_usage();
        self.finish_node();
    }

    // production: RequirementVerificationUsage@sysml
    //
    // RequirementVerificationUsage : RequirementUsage =
    //     ownedRelationship += OwnedReferenceSubsetting FeatureSpecialization*
    //     RequirementBody
    //   | ( UsageExtensionKeyword* 'requirement' | UsageExtensionKeyword+ )
    //     ConstraintUsageDeclaration RequirementBody             (SysML 8.2.2.24)
    //
    // The second alternative is read as RequirementConstraintUsage's is, a `#` standing
    // in for `requirement`. Unlike RequirementConstraintUsage both alternatives take a
    // RequirementBody, and
    // the reference alternative takes `FeatureSpecialization*`, not a
    // FeatureSpecializationPart: no multiplicity (`verify r[1];` is reported), as
    // VariantReference has none. Told apart on one token: `requirement` is reserved, and
    // `#` opens no name.
    fn requirement_verification_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementVerificationUsage);
        if self.at_keyword("requirement") || self.at(SyntaxKind::Hash) {
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.eat_optional_keyword("requirement");
            self.constraint_usage_declaration();
        } else {
            self.owned_reference_subsetting();
            while self.at_feature_specialization() {
                self.feature_specialization();
            }
        }
        self.requirement_body();
        self.finish_node();
    }

    // production: SubjectUsage@sysml
    //
    // SubjectUsage : ReferenceUsage =
    //     'subject' UsageExtensionKeyword* Usage                (SysML 8.2.2.21.1)
    //
    // The keywords follow `subject`, as ActorUsage's follow `actor`.
    //
    // The metaclass is ReferenceUsage, not a SubjectUsage of its own: what makes the
    // usage a subject is the membership that owns it, not the usage.
    fn subject_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SubjectUsage);
        self.expect_keyword("subject");
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.usage();
        self.finish_node();
    }

    // production: CaseDefinition
    // production: AnalysisCaseDefinition
    // production: VerificationCaseDefinition
    // production: UseCaseDefinition
    //
    // CaseDefinition         = OccurrenceDefinitionPrefix 'case' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.22)
    // AnalysisCaseDefinition = OccurrenceDefinitionPrefix 'analysis' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.23)
    // VerificationCaseDefinition = OccurrenceDefinitionPrefix 'verification' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.24)
    // UseCaseDefinition      = OccurrenceDefinitionPrefix 'use' 'case' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.25)
    //
    // One method, the keywords and node from `CASES`. "A case definition or usage is
    // declared as a kind of calculation definition or usage ... using the kind keyword
    // case" (7.22.2, receipt eb25a69f), and an analysis case as a case with the kind
    // keyword analysis (7.23.2, receipt 2aa2d6ce), a verification case with
    // `verification` (7.24.2, receipt d518fc8c), a use case with `use case` (7.25.2,
    // receipt 9be3712a): CalculationDefinition's spine over CaseBody. The metaclasses
    // chain AnalysisCaseDefinition, VerificationCaseDefinition and UseCaseDefinition >
    // CaseDefinition > CalculationDefinition (8.3.23.2, receipt 188d1035; 8.3.24.3,
    // receipt 0cc87426; 8.3.25.3, receipt 32b25e8d; 8.3.22.2, receipt 692a4982).
    //
    // implied specialization: Cases::Case, AnalysisCases::AnalysisCase,
    //     VerificationCases::VerificationCase or UseCases::UseCase
    // constraint: CaseDefinition::checkCaseDefinitionSpecialization,
    //     `specializesFromLibrary('Cases::Case')` (8.3.22.2), and
    //     AnalysisCaseDefinition::checkAnalysisCaseDefinitionSpecialization,
    //     `specializesFromLibrary('AnalysisCases::AnalysisCase')` (8.3.23.2), and
    //     VerificationCaseDefinition::checkVerificationCaseSpecialization,
    //     `specializesFromLibrary('VerificationCases::VerificationCase')` (8.3.24.3), and
    //     UseCaseDefinition::checkUseCaseDefinitionSpecialization,
    //     `specializesFromLibrary('UseCases::UseCase')` (8.3.25.3). Injections, so
    //     sv2-hir's; this layer builds the tree only (ADR-0002).
    // constraint: UseCaseDefinition::deriveUseCaseDefinitionIncludedUseCase (8.3.25.3), a
    //     derivation over the IncludeUseCaseUsages the body owns; sv2-resolve's.
    // constraint: CaseDefinition::validateCaseDefinitionOnlyOneSubject,
    //     validateCaseDefinitionOnlyOneObjective and
    //     validateCaseDefinitionSubjectParameterPosition (8.3.22.2): at most one subject
    //     and one objective, and the subject the first parameter. Constraints on what the
    //     body holds, not grammar, so sv2-resolve's; the body reads any number of each.
    fn case_definition(&mut self, case: Case) {
        self.eat_trivia();
        self.start_node(case.definition);
        self.occurrence_definition_prefix();
        for word in case.keywords {
            self.expect_keyword(word);
        }
        self.expect_keyword("def");
        self.definition_declaration();
        self.case_body();
        self.finish_node();
    }

    // production: CaseUsage
    // production: AnalysisCaseUsage
    // production: VerificationCaseUsage
    // production: UseCaseUsage
    //
    // CaseUsage         = OccurrenceUsagePrefix 'case'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.22)
    // AnalysisCaseUsage = OccurrenceUsagePrefix 'analysis'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.23)
    // VerificationCaseUsage = OccurrenceUsagePrefix 'verification'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.24)
    // UseCaseUsage      = OccurrenceUsagePrefix 'use' 'case'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.25)
    //
    // ConstraintUsageDeclaration and not ActionUsageDeclaration, although a CaseUsage is a
    // CalculationUsage (8.3.22.3, receipt f0ff6680) and CalculationUsage takes the
    // action's: the grammar states it so. The two bodies are the same,
    // `UsageDeclaration ValuePart?` (8.2.2.20, 8.2.2.17.2), so the choice decides the node
    // name alone and not what text is read.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Cases::cases, AnalysisCases::analysisCases,
    //     VerificationCases::verificationCases or UseCases::useCases
    // constraint: CaseUsage::checkCaseUsageSpecialization (8.3.22.3),
    //     AnalysisCaseUsage::checkAnalysisCaseUsageSpecialization (8.3.23.3, receipt
    //     ac8c6d0a), VerificationCaseUsage::checkVerificationCaseUsageSpecialization
    //     (8.3.24.4, receipt 980c6c7a) and UseCaseUsage::checkUseCaseUsageSpecialization
    //     (8.3.25.4, receipt b7b869b1), and the composite-owned
    //     checkCaseUsageSubcaseSpecialization,
    //     checkAnalysisCaseUsageSubAnalysisCaseSpecialization,
    //     checkVerificationCaseUsageSubVerificationCaseSpecialization and
    //     checkUseCaseUsageSubUseCaseSpecialization. sv2-hir's (ADR-0002).
    fn case_usage(&mut self, case: Case) {
        self.eat_trivia();
        self.start_node(case.usage);
        self.occurrence_usage_prefix();
        for word in case.keywords {
            self.expect_keyword(word);
        }
        self.constraint_usage_declaration();
        self.case_body();
        self.finish_node();
    }

    // production: CaseBody
    //
    // CaseBody : Type =
    //     ';'
    //   | '{' CaseBodyItem* ( ownedRelationship += ResultExpressionMember )? '}'
    //                                                            (SysML 8.2.2.22)
    //
    // CalculationBody's shape with the part inlined, so there is no CaseBodyPart node:
    // the grammar names none. The items are read by `body_elements` under `Body::Case`,
    // which says what they are; the trailing expression as `calculation_body_part` reads
    // its own.
    //
    // CaseBodyItem is marked at `body_element`:
    //
    //     CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember | ObjectiveMember
    //
    // all four read under Body::Case, and `return` too, by deviation CaseBodyItem; see
    // `Body::admits_return_parameter`.
    fn case_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CaseBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Case);
            // As in `calculation_body_part`: the loop returned before the `}` only
            // because what is left is the trailing ResultExpressionMember.
            if !self.at_end() && !self.at(SyntaxKind::RBrace) {
                self.result_expression_member();
            }
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a case declaration");
        }
        self.finish_node();
    }

    // production: ObjectiveMember
    //
    // ObjectiveMember : ObjectiveMembership =
    //     MemberPrefix 'objective'
    //     ownedRelatedElement += ObjectiveRequirementUsage       (SysML 8.2.2.22)
    //
    // "The objective of a case definition or usage is declared as a requirement usage
    // ..., but using the keyword objective instead of requirement" (7.22.2, receipt
    // eb25a69f). Unlike SubjectMember, the keyword belongs to the MEMBER here, not to the
    // usage it owns. The metaclass is ObjectiveMembership (8.3.22.4, receipt d68190c3), a
    // FeatureMembership.
    //
    // constraint: ObjectiveMembership::validateObjectiveMembershipOwningType (8.3.22.4):
    //     `owningType.oclIsType(CaseDefinition) or owningType.oclIsType(CaseUsage)`. The
    //     grammar says only part of it: CaseBodyItem alone reaches this, but the analysis,
    //     verification and use case bodies reach CaseBodyItem too, and the corpus writes
    //     objectives there (AnalysisTest.sysml, Annex A's use cases), which an exact-type
    //     test refuses. sv2-resolve's to decide, as a spec question and not this layer's.
    // constraint: ObjectiveMembership::validateObjectiveMembershipIsComposite (8.3.22.4).
    //     sv2-resolve's; the tree carries no `ref` here to contradict it.
    fn objective_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ObjectiveMember);
        self.member_prefix();
        self.expect_keyword("objective");
        self.objective_requirement_usage();
        self.finish_node();
    }

    // production: ObjectiveRequirementUsage@sysml
    //
    // ObjectiveRequirementUsage : RequirementUsage =
    //     UsageExtensionKeyword* ConstraintUsageDeclaration RequirementBody
    //                                                            (SysML 8.2.2.22)
    //
    // `objective #goal o;` carries one. A RequirementBody, so a `require` or
    // `subject` inside an objective reads as it does inside a requirement; "the subject of
    // an objective requirement is bound by default to the result" (7.22.2) is a binding
    // sv2-hir injects, not text.
    fn objective_requirement_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ObjectiveRequirementUsage);
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.constraint_usage_declaration();
        self.requirement_body();
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
    //
    // Package = ( ownedRelationship += PrefixMetadataMember )*
    //           PackageDeclaration PackageBody                   (SysML 8.2.2.5.1)
    //
    // The PrefixMetadataMembers stand directly in the package, with no extension-keyword
    // node between, as the clause writes them. KerML's Package states the same text over
    // its own PrefixMetadataMember (8.2.5.13, 8.2.5.12), which `prefix_metadata_member`
    // reads in the file's language.
    fn package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Package);
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
        self.package_declaration();
        self.package_body();
        self.finish_node();
    }

    // production: LibraryPackage
    //
    // LibraryPackage =
    //     ( isStandard ?= 'standard' ) 'library'
    //     ( ownedRelationship += PrefixMetadataMember )*
    //     PackageDeclaration PackageBody          (SysML 8.2.2.5.1, KerML 8.2.5.13)
    //
    // Read as deviation LibraryPackage (follow_xtext) writes it: `'standard'?`. The printed
    // group has no `?`, which would make every library package standard, where KerML 7.4.14
    // writes `library package AddressBooks {` and the LibraryPackage metaclass says
    // isStandard "should only be set to true" for recognised standard libraries
    // (8.3.4.13.3). So `library package` without `standard` is the text the deviation adds,
    // and it carries the note; the corpus writes nothing else.
    //
    // One unit in both grammars, whose texts are the same (ADR-0015). The
    // PrefixMetadataMember* is each language's own, PrefixMetadataMember@sysml or
    // PrefixMetadataMember@kerml, read by `prefix_metadata_member`.
    fn library_package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LibraryPackage);
        if self.at_keyword("standard") {
            self.bump_as(keyword("standard").unwrap_or(SyntaxKind::BasicName));
        } else {
            // deviation: LibraryPackage
            self.note_deviation("LibraryPackage", "a library package that is not `standard`");
        }
        self.expect_keyword("library");
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
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

    // production: PackageBody@kerml
    // production: PackageBody@sysml
    //
    // PackageBody = ';' | '{' PackageBodyElement* '}'              (SysML 8.2.2.5.1)
    // PackageBody = ';' | '{' ( NamespaceBodyElement
    //                         | ElementFilterMember )* '}'         (KerML 8.2.5.13)
    //
    // Two grammar units, one method: `Body::Package` asks `body_elements` for the
    // language's own items, and admits ElementFilterMember in both, where both grammars
    // put it. Marked on DefinitionBody's convention — the body is read in full, its item
    // productions only in part.
    fn package_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PackageBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Package);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a package declaration");
        }
        self.finish_node();
    }
}

/// Parse `source` against `language`'s grammar into a lossless tree.
///
/// `KerML` and `SysML` are two grammars with two start symbols, not one grammar that
/// extends the other, so the caller says which is meant (ADR-0014). There is no default:
/// reading a file against the grammar its author did not write it in accepts constructs
/// the language does not have, and a positive-only corpus sweep cannot see that happen.
/// [`Language::from_path`] is how a caller holding a path chooses.
///
/// Never fails and never panics: text no production accepts becomes `Error` nodes
/// that keep their bytes, so `parse(s, l).text() == s` for every `s` and every `l`.
#[must_use]
pub fn parse(source: &str, language: Language) -> Parse {
    let (green, errors, deviations) = Parser::new(source, language).root_namespace();
    Parse {
        green,
        errors,
        deviations,
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, Parser, SyntaxKind};
    use crate::parser::lookahead::{VISIBILITY, keyword};

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
        "merge",
        "decide",
        "join",
        "fork",
        "specializes",
        "comment",
        "about",
        "locale",
        "doc",
        "rep",
        "language",
        // The keywords a KerML FeaturePrefix stands before. `nth_continues_feature`
        // counts a NAME as continuing a feature, so `feature_prefix` tells the `#` before
        // one of these from the `#` in `feature`'s place only while each is reserved
        // (KerML 8.2.2.6) and so no NAME.
        "feature",
        "connector",
        "succession",
        "binding",
        "step",
        "expr",
        "inv",
        "flow",
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

    /// `SysML`'s `ExpressionBody` is `CalculationBody`'s braced alternative alone
    /// (decision expression-body-semicolon). Every caller dispatches on `{`, so no text reaches
    /// `body_expression` at a `;`; this drives it there directly, so the narrowing is
    /// held by the production and not only by its callers.
    #[test]
    fn a_sysml_expression_body_does_not_take_the_semicolon_alternative() {
        let mut braced = Parser::new("{ x }", Language::SysMl);
        braced.body_expression();
        assert!(braced.errors.is_empty());

        let mut bare = Parser::new(";", Language::SysMl);
        bare.body_expression();
        assert!(!bare.errors.is_empty());

        // The calculation body a `calc def` owns still takes it (SysML 8.2.2.19).
        let mut calc = Parser::new(";", Language::SysMl);
        calc.calculation_body();
        assert!(calc.errors.is_empty());
    }

    /// The meaningful-token index answers exactly what filtering the tokens from the
    /// cursor answered, at every cursor, every distance and in both comment modes,
    /// including past the end. Text with both comment forms, a note, whitespace runs and
    /// an unterminated comment, so every kind of trivia sits somewhere in it.
    #[test]
    fn the_meaningful_index_agrees_with_filtering_from_the_cursor() {
        let source = "package /* c */ P { // n\n  part   a : A; /* d */ doc /* e */ } /* open";
        let mut parser = Parser::new(source, Language::SysMl);
        let total = parser.tokens.len();
        for significant in [false, true] {
            parser.comments_significant = significant;
            for pos in 0..=total + 1 {
                parser.pos = pos;
                assert_lookahead_matches_filter(&parser);
            }
        }
    }

    /// Every distance from the parser's cursor, against the filter `peek_nth` replaced.
    fn assert_lookahead_matches_filter(parser: &Parser<'_>) {
        let expected: Vec<_> = parser
            .tokens
            .get(parser.pos..)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|token| !parser.skippable(token.kind))
            .collect();
        for n in 0..=expected.len() + 1 {
            assert_eq!(parser.peek_nth(n), expected.get(n).copied());
            let from: Vec<_> = parser.meaningful_from(n).collect();
            assert_eq!(from, expected.get(n..).unwrap_or(&[]));
        }
    }
}
