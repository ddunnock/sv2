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
mod calculation;
mod case;
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
mod requirement;
mod state;
mod succession;
mod tree;
mod usage;
mod view;

use std::cell::Cell;

use rowan::{GreenNode, GreenNodeBuilder};

use crate::diagnostic::Diagnostic;
use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::language::SyntaxNode;
use crate::lexer::{Token, is_trivia, tokenize};
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;
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
