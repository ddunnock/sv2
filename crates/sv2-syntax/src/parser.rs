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
//! operator core reads all fifteen tiers; the postfix layer of 8.2.5.8.2 — `a.b`,
//! `x[kg]`, `x#(1)`, `x->f()` — and `BodyExpression` are not implemented, each with a
//! rejection case in `tests/rejection/` naming its clause.
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

use rowan::{GreenNode, GreenNodeBuilder, Language as _};
use text_size::{TextRange, TextSize};

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::generated::kinds::{KEYWORDS, OPERATORS, SyntaxKind};
use crate::grammar::Language;
use crate::language::{Sv2Language, SyntaxNode};
use crate::lexer::{Token, is_trivia, is_unterminated_comment, tokenize};

/// The result of parsing: a tree, plus what went wrong.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<Diagnostic>,
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

/// The membership an operand of a postfix expression is owned through.
///
/// `PrimaryArgumentMember = ownedMemberParameter = PrimaryArgument`, `PrimaryArgument =
/// ownedRelationship += PrimaryArgumentValue`, `PrimaryArgumentValue = value =
/// PrimaryExpression` (`KerML` 8.2.5.8.2). Three productions and no tokens, wrapped
/// around an operand that is already in the tree.
const PRIMARY_ARGUMENT: [SyntaxKind; 3] = [
    SyntaxKind::PrimaryArgumentMember,
    SyntaxKind::PrimaryArgument,
    SyntaxKind::PrimaryArgumentValue,
];

/// The same three for a `FeatureChainExpression`, whose member has its own name.
///
/// Only the outermost differs. `NonFeatureChainPrimaryArgumentMember`'s body is
/// `PrimaryArgument` despite the name, which is what makes the chain fold left — see
/// `feature_chain_expression`.
const NON_FEATURE_CHAIN_PRIMARY_ARGUMENT: [SyntaxKind; 3] = [
    SyntaxKind::NonFeatureChainPrimaryArgumentMember,
    SyntaxKind::PrimaryArgument,
    SyntaxKind::PrimaryArgumentValue,
];

/// The three `VisibilityIndicator` keywords, in the order the specification
/// writes them (`SysML` 8.2.2.5.1). Looked up in the pinned token set like every
/// other keyword; this is only the list of which ones the production names.
const VISIBILITY: [&str; 3] = ["public", "private", "protected"];

// -- the precedence table, KerML 8.2.5.8.1 table 6 -------------------------------
//
// PRECEDENCE IS NOT IN THE GRAMMAR AND CANNOT BE. KerML 8.2.5.8.1 note 2 states that
// the grouping of nested OperatorExpressions is not expressed in the productions and
// is determined by the precedence of the operators as given in table 6, and that every
// BinaryOperator groups to the left except exponentiation, which groups to the right.
//
// The table is recorded as data at docs/operator-precedence.toml, cited to that clause
// and table. What follows is that file's tiers in the form the parser reads them, and
// `the_infix_table_is_the_recorded_precedence_table` holds the two against each other
// so neither can be edited alone.

/// How a tier groups when the same tier repeats.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Assoc {
    /// `a - b - c` is `(a - b) - c`. Every binary tier but exponentiation.
    Left,
    /// `a ** b ** c` is `a ** (b ** c)`. Exponentiation alone, by note 2.
    Right,
}

/// How an operator is written: a symbol the lexer gives its own kind, or a word that
/// arrives as a `BasicName` and is separated from a name by the pinned keyword table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Spelling {
    Symbol(SyntaxKind),
    Word(&'static str),
}

/// The membership an operand is owned through, which is not the same for every operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Operand {
    /// `ArgumentMember` — an expression, evaluated as an argument.
    Argument,
    /// `ArgumentExpressionMember` — an expression, referenced rather than evaluated.
    /// This is what makes `??`, `or`, `and` and `implies` short-circuit where `|`,
    /// `&` and `xor` do not.
    ArgumentExpression,
    /// `TypeReferenceMember` — a type, named by a classification test.
    TypeReference,
    /// `TypeResultMember` — a type, named by a cast, whose type is its result.
    TypeResult,
}

/// One infix operator: its spelling, its tier, how it groups, and what it builds.
struct InfixOperator {
    spelling: Spelling,
    /// The tier from table 6. Lower binds tighter.
    tier: u8,
    assoc: Assoc,
    /// The node this operator builds.
    node: SyntaxKind,
    /// The memberships the left operand is wrapped in, outermost first.
    left: &'static [SyntaxKind],
    /// The membership the right operand is read into.
    right: Operand,
}

/// The membership chain an `ArgumentMember` left operand is wrapped in.
const LEFT_ARGUMENT: &[SyntaxKind] = &[
    SyntaxKind::ArgumentMember,
    SyntaxKind::Argument,
    SyntaxKind::ArgumentValue,
];

/// Tier 2, `UnaryOperatorExpression`. TIGHTER than exponentiation at tier 3, so
/// `-2 ** 2` is `(-2) ** 2` — the opposite of the C and Python convention.
const TIER_UNARY: u8 = 2;
/// Tier 8, the classification and metaclassification operators.
const TIER_CLASSIFICATION: u8 = 8;
/// Tier 15, `ConditionalExpression`. The loosest tier in table 6.
const TIER_CONDITIONAL: u8 = 15;
/// The loosest tier an expression may open at, which is every tier.
const TIER_LOOSEST: u8 = TIER_CONDITIONAL;

/// `UnaryOperator = '+' | '-' | '~' | 'not'` (`KerML` 8.2.5.8.1), tier 2.
const UNARY_OPERATORS: [Spelling; 4] = [
    Spelling::Symbol(SyntaxKind::Plus),
    Spelling::Symbol(SyntaxKind::Minus),
    Spelling::Symbol(SyntaxKind::Tilde),
    Spelling::Word("not"),
];

/// Every infix operator, tightest tier first.
///
/// Order matters only within a tier, where it does not decide anything: the spellings
/// are disjoint, so `infix_operator_here` finds the one that is written. Tiers 3 to 14
/// of table 6; tiers 1, 2 and 15 are prefix and are not here.
///
/// Three tiers mix two productions, and the difference is the right operand:
///   tier 10  `&` is a `BinaryOperator` and `and` a `ConditionalBinaryOperator`
///   tier 12  `|` is a `BinaryOperator` and `or`  a `ConditionalBinaryOperator`
///   tier  8  `istype`, `hastype`, `@` test and `as` casts, into different memberships
const INFIX: [InfixOperator; 27] = [
    // Tier 3 — exponentiation, the one right-associative tier (note 2).
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Caret),
        tier: 3,
        assoc: Assoc::Right,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::StarStar),
        tier: 3,
        assoc: Assoc::Right,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 4 — multiplicative.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Star),
        tier: 4,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Slash),
        tier: 4,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Percent),
        tier: 4,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 5 — additive.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Plus),
        tier: 5,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Minus),
        tier: 5,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 6 — range. Left-associative by note 2, which is more permissive than the
    // Pilot: its RangeExpression rule takes at most one `..` and rejects `a..b..c`.
    // That is a property of its parser generator, not of the language.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::DotDot),
        tier: 6,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 7 — relational.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::LtEq),
        tier: 7,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::GtEq),
        tier: 7,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Lt),
        tier: 7,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Gt),
        tier: 7,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 8 — classification. The right operand is a TYPE, not an expression, and a
    // test and a cast name it through different memberships. `@@` and `meta` share
    // the tier but not the left operand, so they are not here: see
    // `metaclassification_expression`.
    InfixOperator {
        spelling: Spelling::Word("istype"),
        tier: TIER_CLASSIFICATION,
        assoc: Assoc::Left,
        node: SyntaxKind::ClassificationExpression,
        left: LEFT_ARGUMENT,
        right: Operand::TypeReference,
    },
    InfixOperator {
        spelling: Spelling::Word("hastype"),
        tier: TIER_CLASSIFICATION,
        assoc: Assoc::Left,
        node: SyntaxKind::ClassificationExpression,
        left: LEFT_ARGUMENT,
        right: Operand::TypeReference,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::At),
        tier: TIER_CLASSIFICATION,
        assoc: Assoc::Left,
        node: SyntaxKind::ClassificationExpression,
        left: LEFT_ARGUMENT,
        right: Operand::TypeReference,
    },
    InfixOperator {
        spelling: Spelling::Word("as"),
        tier: TIER_CLASSIFICATION,
        assoc: Assoc::Left,
        node: SyntaxKind::ClassificationExpression,
        left: LEFT_ARGUMENT,
        right: Operand::TypeResult,
    },
    // Tier 9 — equality.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::EqEqEq),
        tier: 9,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::BangEqEq),
        tier: 9,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::EqEq),
        tier: 9,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::BangEq),
        tier: 9,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 10 — `&` conjoins and `and` short-circuits.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Amp),
        tier: 10,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Word("and"),
        tier: 10,
        assoc: Assoc::Left,
        node: SyntaxKind::ConditionalBinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::ArgumentExpression,
    },
    // Tier 11 — exclusive or, which has no short-circuiting spelling.
    InfixOperator {
        spelling: Spelling::Word("xor"),
        tier: 11,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    // Tier 12 — `|` disjoins and `or` short-circuits.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::Pipe),
        tier: 12,
        assoc: Assoc::Left,
        node: SyntaxKind::BinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::Argument,
    },
    InfixOperator {
        spelling: Spelling::Word("or"),
        tier: 12,
        assoc: Assoc::Left,
        node: SyntaxKind::ConditionalBinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::ArgumentExpression,
    },
    // Tier 13 — implication.
    InfixOperator {
        spelling: Spelling::Word("implies"),
        tier: 13,
        assoc: Assoc::Left,
        node: SyntaxKind::ConditionalBinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::ArgumentExpression,
    },
    // Tier 14 — null coalescing, the loosest infix tier.
    InfixOperator {
        spelling: Spelling::Symbol(SyntaxKind::QuestionQuestion),
        tier: 14,
        assoc: Assoc::Left,
        node: SyntaxKind::ConditionalBinaryOperatorExpression,
        left: LEFT_ARGUMENT,
        right: Operand::ArgumentExpression,
    },
];

/// The infix table as `(tier, operator, associativity)`, for the test that holds it
/// against `docs/operator-precedence.toml`.
///
/// Exposed because an integration test reaches only the public API, and the check it
/// makes is one of the more valuable in the crate: that the parser groups operators
/// by the table the specification cites, rather than by a table that has drifted
/// from it. `KerML` 8.2.5.8.1 note 2 is the reason the two files exist separately at
/// all.
///
/// A symbol's text comes from `OPERATORS`, the pinned token set, rather than being
/// written out again here — so an operator leaving the token set fails this test too.
#[doc(hidden)]
#[must_use]
pub fn infix_table_for_test() -> Vec<(u8, String, String)> {
    INFIX
        .iter()
        .map(|op| {
            let operator = match op.spelling {
                Spelling::Symbol(kind) => OPERATORS
                    .iter()
                    .find(|(_, k)| *k == kind)
                    .map_or_else(String::new, |(text, _)| (*text).to_owned()),
                Spelling::Word(word) => word.to_owned(),
            };
            let assoc = match op.assoc {
                Assoc::Left => "left",
                Assoc::Right => "right",
            };
            (op.tier, operator, assoc.to_owned())
        })
        .collect()
}

/// A usage production whose whole rule is `<prefix> KEYWORD Usage`.
#[derive(Clone, Copy)]
struct SimpleUsage {
    /// The one keyword that says which production this is.
    keyword: &'static str,
    /// The node the production builds.
    node: SyntaxKind,
    /// Whether the prefix is an `OccurrenceUsagePrefix` rather than a `UsagePrefix`.
    ///
    /// An occurrence may carry `'individual'` and a `PortionKind` (`SysML` 8.2.2.9.2);
    /// an attribute and an enumeration may not, because neither is an occurrence.
    is_occurrence: bool,
    /// Which of 8.2.2.6.4's three sorts of usage this is, which decides the membership
    /// a body owns it through. Stated per usage rather than read off `is_occurrence`:
    /// the two agree for these seven, but one is about the prefix and the other about
    /// the element, and `ActionUsage` is an occurrence whose class is `Behavior`.
    class: UsageClass,
}

/// The three sorts of usage `SysML` 8.2.2.6.4 names, each with its own element
/// alternation and so its own membership.
///
/// ```text
/// NonOccurrenceUsageElement = DefaultReferenceUsage | ReferenceUsage | AttributeUsage
///                           | EnumerationUsage | BindingConnectorAsUsage
///                           | SuccessionAsUsage | ExtendedUsage
/// StructureUsageElement     = OccurrenceUsage | IndividualUsage | PortionUsage
///                           | EventOccurrenceUsage | ItemUsage | PartUsage | ViewUsage
///                           | RenderingUsage | PortUsage | ConnectionUsage | ...
/// BehaviorUsageElement      = ActionUsage | CalculationUsage | StateUsage | ...
///                           | PerformActionUsage | ...
/// ```
///
/// `OccurrenceUsageElement = StructureUsageElement | BehaviorUsageElement`, so a
/// definition body, which asks only occurrence or not, does not tell the last two apart
/// and an action body does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum UsageClass {
    NonOccurrence,
    Structure,
    Behavior,
}

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

/// The four `ControlNode`s: `ControlNodePrefix KEYWORD UsageDeclaration ActionBody`,
/// differing in the keyword and the metaclass (`SysML` 8.2.2.17.3). The keywords are
/// reserved and disjoint, so the order decides nothing.
const CONTROL_NODES: [(&str, SyntaxKind); 4] = [
    ("merge", SyntaxKind::MergeNode),
    ("decide", SyntaxKind::DecisionNode),
    ("join", SyntaxKind::JoinNode),
    ("fork", SyntaxKind::ForkNode),
];

/// Every usage production that is a prefix, one keyword and the `Usage` spine.
///
/// Ordered as the clauses number them. The keywords are disjoint, so the order does
/// not decide anything — `at_simple_usage` takes the one whose keyword is written.
const SIMPLE_USAGES: [SimpleUsage; 7] = [
    SimpleUsage {
        keyword: "attribute",
        node: SyntaxKind::AttributeUsage,
        is_occurrence: false,
        class: UsageClass::NonOccurrence,
    },
    SimpleUsage {
        keyword: "enum",
        node: SyntaxKind::EnumerationUsage,
        is_occurrence: false,
        class: UsageClass::NonOccurrence,
    },
    SimpleUsage {
        keyword: "occurrence",
        node: SyntaxKind::OccurrenceUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "item",
        node: SyntaxKind::ItemUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "part",
        node: SyntaxKind::PartUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "port",
        node: SyntaxKind::PortUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "rendering",
        node: SyntaxKind::RenderingUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
];

/// Which body production is being read, and so which elements it admits.
///
/// The two grammars disagree about the root and about what a package body holds, and
/// the disagreement is not the same shape in both places (ADR-0014):
///
/// ```text
/// RootNamespace@sysml  = PackageBodyElement*                          SysML 8.2.2.5.1
/// PackageBody@sysml    = ';' | '{' PackageBodyElement* '}'             SysML 8.2.2.5.1
/// RootNamespace@kerml  = NamespaceBodyElement*                        KerML 8.2.3.4.1
/// PackageBody@kerml    = ';' | '{' ( NamespaceBodyElement
///                                  | ElementFilterMember )* '}'       KerML 8.2.3.4.1
/// DefinitionBody@sysml = ';' | '{' DefinitionBodyItem* '}'            SysML 8.2.2.6.1
/// ```
///
/// Two questions come out of that, and they do NOT line up, which is why they are asked
/// separately rather than one being derived from the other.
///
/// The membership node differs by language: `PackageBodyElement` reaches `PackageMember`
/// and `NamespaceBodyElement` reaches `NonFeatureMember`.
///
/// Whether a filter is admitted differs by BOTH. An `ElementFilterMember` is a
/// `PackageBodyElement`, so `SysML` admits one at the root and in a package body alike. In
/// `KerML` it is not a `NamespaceBodyElement` at all — `PackageBody` adds it, and the root
/// does not. So a filter is admitted in a `KerML` package body and refused at a `KerML` root,
/// and deriving that from the membership node would accept `filter` where `KerML` has none.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    /// `RootNamespace`: the whole file.
    Root,
    /// The braced form of `PackageBody`.
    Package,
    /// The braced form of `DefinitionBody`. `SysML` only — `KerML` has no definitions.
    Definition,
    /// The braced form of `RequirementBody`. `SysML` only.
    ///
    /// `RequirementBodyItem = DefinitionBodyItem | SubjectMember | …` (8.2.2.21.1), so
    /// every question `Body::Definition` answers this answers the same way. It is a
    /// variant of its own for the ONE thing it answers differently: the six extra
    /// members, of which `SubjectMember` is implemented. A `subject` reached from a
    /// definition body is not a `SubjectMember` — `DefinitionBodyItem` has no such
    /// alternative — and without this variant it would be read as one.
    Requirement,
    /// The braced form of `ActionBody`. `SysML` only.
    ///
    /// ```text
    /// ActionBodyItem = NonBehaviorBodyItem
    ///                | InitialNodeMember ActionTargetSuccessionMember*
    ///                | SourceSuccessionMember? ActionBehaviorMember
    ///                  ActionTargetSuccessionMember*
    ///                | GuardedSuccessionMember                    SysML 8.2.2.17.1
    /// ```
    ///
    /// `NonBehaviorBodyItem`'s implemented alternatives — `Import`, `AliasMember`,
    /// `DefinitionMember` — are the three a definition body reads, so for those this
    /// answers as `Definition` does. It differs in the control-flow alternatives, of
    /// which `InitialNodeMember` is implemented: see `admits_action_body_item`.
    ///
    /// It was a variant before it decided anything, because the ITEM SET differs in the
    /// grammar even where the implemented part did not. That is what let `first` attach
    /// here with no change to the dispatch of anything else — the second time a body
    /// variant argued to be identical to `Definition` stopped being so one production
    /// later, `requirement_body` and `SubjectMember` being the first.
    Action,
    /// The item run inside the braced form of `CalculationBody`. `SysML` only.
    ///
    /// `CalculationBodyItem = ActionBodyItem | ReturnParameterMember` (8.2.2.19), and
    /// `ActionBodyItem`'s first alternative is `NonBehaviorBodyItem = Import |
    /// AliasMember | DefinitionMember | VariantUsageMember | NonOccurrenceUsageMember |
    /// SourceSuccessionMember? StructureUsageMember` (8.2.2.17.1). Of those, the same
    /// three a definition body reads are implemented, which is why this shares the loop.
    ///
    /// It is a variant of its own because the loop must STOP before the trailing
    /// `ResultExpressionMember`, and no other body has one. See `at_result_expression`.
    Calculation,
    /// The braced form of `TypeBody`. `KerML` only — what a classifier holds.
    Type,
}

impl Body {
    /// The membership node a nested element is owned through.
    ///
    /// In a `SysML` definition or action body the answer depends on the ELEMENT, not only
    /// on the body: `DefinitionMember` owns definitions alone (8.2.2.6.1), and a usage is
    /// owned through the membership its body's item production names for its class.
    /// Until this distinction was drawn every usage in these bodies was built as a
    /// `DefinitionMember`, a node the grammar gives no usage.
    fn member(self, language: Language, element: MemberElement) -> SyntaxKind {
        match (self, language, element) {
            // A requirement body owns its DefinitionBodyItem alternative exactly as a
            // definition body does; what it owns differently owns itself, through
            // SubjectMember. A calculation body reaches DefinitionMember too, by the
            // other road: CalculationBodyItem to ActionBodyItem to NonBehaviorBodyItem,
            // whose third alternative it is (8.2.2.17.1).
            (
                Self::Definition | Self::Requirement | Self::Calculation | Self::Action,
                _,
                MemberElement::Other,
            ) => SyntaxKind::DefinitionMember,
            // Both DefinitionBodyItem (8.2.2.6.1) and NonBehaviorBodyItem (8.2.2.17.1)
            // name NonOccurrenceUsageMember.
            (
                Self::Definition | Self::Requirement | Self::Calculation | Self::Action,
                _,
                MemberElement::Usage(UsageClass::NonOccurrence),
            ) => SyntaxKind::NonOccurrenceUsageMember,
            // DefinitionBodyItem: `SourceSuccessionMember? OccurrenceUsageMember`
            // (8.2.2.6.1), and OccurrenceUsageElement is both of the other classes.
            (Self::Definition | Self::Requirement, _, MemberElement::Usage(_)) => {
                SyntaxKind::OccurrenceUsageMember
            }
            // NonBehaviorBodyItem: `SourceSuccessionMember? StructureUsageMember`
            // (8.2.2.17.1).
            (Self::Calculation | Self::Action, _, MemberElement::Usage(UsageClass::Structure)) => {
                SyntaxKind::StructureUsageMember
            }
            // ActionBodyItem's third alternative: `SourceSuccessionMember?
            // ActionBehaviorMember ActionTargetSuccessionMember*`, and
            // ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember (8.2.2.17.1).
            // Only the member is read here; `source_succession_item` reads the `then`
            // before it and `behaviour_targets` the target successions after it.
            (Self::Calculation | Self::Action, _, MemberElement::Usage(UsageClass::Behavior)) => {
                SyntaxKind::BehaviorUsageMember
            }
            (Self::Calculation | Self::Action, _, MemberElement::ActionNode) => {
                SyntaxKind::ActionNodeMember
            }
            (_, Language::SysMl, _) => SyntaxKind::PackageMember,
            // Both KerML bodies own the same membership. `TypeBodyElement` is
            // `NonFeatureMember | FeatureMember | AliasMember | Import` (8.2.4.1) and
            // `NamespaceBodyElement` reaches `NonFeatureMember` too (8.2.3.4.1), so
            // `Body::Type` needs no arm of its own. `FeatureMember` is unimplemented in
            // both, which is what leaves them identical for now rather than by rule.
            (_, Language::KerMl, _) => SyntaxKind::NonFeatureMember,
        }
    }

    /// Whether `ElementFilterMember` is one of this body's alternatives.
    fn admits_filter(self, language: Language) -> bool {
        match self {
            Self::Package => true,
            Self::Root => language == Language::SysMl,
            // TypeBodyElement has no ElementFilterMember alternative, and neither
            // DefinitionBodyItem, RequirementBodyItem nor NonBehaviorBodyItem reaches one.
            Self::Definition
            | Self::Requirement
            | Self::Calculation
            | Self::Action
            | Self::Type => false,
        }
    }

    /// Whether `SubjectMember` is one of this body's alternatives.
    ///
    /// Only `RequirementBodyItem` reaches it (`SysML` 8.2.2.21.1). `CaseBodyItem` does
    /// too, and `CaseBody` is unimplemented, so this is the whole of it today.
    ///
    /// Asked separately from `member`, for the reason `admits_filter` is: what a body
    /// owns its ordinary members through and which extra alternatives it has are two
    /// questions, and deriving one from the other admits a `subject` in a definition
    /// body — which `DefinitionBodyItem` does not have.
    fn admits_subject(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether `RequirementConstraintMember` is one of this body's alternatives.
    ///
    /// Asked separately from `admits_subject` although both answer `Requirement` today,
    /// because they stop agreeing the moment `CaseBody` lands: `CaseBodyItem` reaches
    /// `SubjectMember` and does NOT reach `RequirementConstraintMember` (`SysML`
    /// 8.2.2.21.1 against 8.2.2.22). Folding them into one question now would have to be
    /// unfolded then, and the unfolding is the kind that gets missed.
    fn admits_requirement_constraint(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether this body's items may be followed by a `ResultExpressionMember`.
    ///
    /// `CalculationBodyPart = CalculationBodyItem* ResultExpressionMember?`
    /// (`SysML` 8.2.2.19), and no other implemented body ends in an expression. The item
    /// loop has to stop before it, because an expression is not a member and the loop
    /// would otherwise recover over it one token at a time.
    fn ends_in_result_expression(self) -> bool {
        matches!(self, Self::Calculation)
    }

    /// Whether `ReturnParameterMember` is one of this body's alternatives.
    ///
    /// `CalculationBodyItem = ActionBodyItem | ReturnParameterMember`
    /// (`SysML` 8.2.2.19), and it is the second alternative — so the containment runs
    /// from calculation to action and NOT the other way. `Body::Action` and
    /// `Body::Calculation` share the item loop, which is exactly how a `return` could
    /// come to be admitted in an action body by accident;
    /// tests/rejection/return-parameter-member-is-not-an-action-body-item.sysml is the
    /// file that fails if it ever is.
    fn admits_return_parameter(self) -> bool {
        matches!(self, Self::Calculation)
    }

    /// Whether `ActionBodyItem`'s alternatives beyond `NonBehaviorBodyItem` belong to
    /// this body — today, `InitialNodeMember`.
    ///
    /// `ActionBodyItem` is reached by eight productions: `ActionBody` (`SysML`
    /// 8.2.2.17.1), `CalculationBodyItem` (8.2.2.19), `CaseBodyItem` (8.2.2.22),
    /// `ActionBodyParameter` (8.2.2.17.7), and the braced bodies of the four
    /// `Transition*ActionUsage`s (8.2.2.18.3). NOT by `RequirementBodyItem`, which reaches
    /// only `DefinitionBodyItem` (8.2.2.21.1), and NOT by `DefinitionBodyItem` (8.2.2.6.1).
    /// A comment on `admits_return_parameter` once said the opposite; it was false, and
    /// widening this to `Requirement` on its word would have admitted `first` where the
    /// grammar has none. tests/rejection/initial-node-member-is-not-a-requirement-body-item.sysml
    /// fails if it ever is. Of the eight, only `ActionBody` and `CalculationBody` have a
    /// `Body` variant — the rest are unimplemented — so `Action` and `Calculation` are the
    /// whole of it today, and each of the others joins this when its body lands.
    ///
    /// `Calculation` covers `ConstraintDefinition` too, which shares `CalculationBody`
    /// (8.2.2.20): a constraint body admits `first` by the grammar, however unusual.
    fn admits_action_body_item(self) -> bool {
        matches!(self, Self::Action | Self::Calculation)
    }

    /// Whether `SourceSuccessionMember` may prefix a member here.
    ///
    /// `DefinitionBodyItem` puts it before `OccurrenceUsageMember` (8.2.2.6.1), and a
    /// requirement body reaches `DefinitionBodyItem` (8.2.2.21.1); `NonBehaviorBodyItem`
    /// puts it before `StructureUsageMember` and `ActionBodyItem` before
    /// `ActionBehaviorMember` (8.2.2.17.1), both reached from action and calculation
    /// bodies. `PackageBodyElement` (8.2.2.5.1) has no such alternative, nor has `KerML`.
    ///
    /// NOT only action bodies, although 7.17.4 says its shorthands "may be used only
    /// within the body of an action definition or usage" (receipt 339ef468). That
    /// sentence scopes the ACTION shorthands of that clause; `DefinitionBodyItem` states
    /// `then` before any occurrence usage unconditionally, and the corpus uses it so:
    /// `then snapshot part vehicle_1_t1 {` inside an `individual part` (training/28.
    /// Individuals/Individuals and Roles-1.sysml:18), and `then event occurrence …` in
    /// the Occurrences examples of training/27.
    fn admits_source_succession(self) -> bool {
        matches!(
            self,
            Self::Definition | Self::Requirement | Self::Action | Self::Calculation
        )
    }
}

/// A `SysML` definition production: one keyword over a shared spine.
///
/// Eight productions of `SysML` 8.2.2 are stated as `<prefix> KEYWORD 'def' Definition`,
/// differing in the keyword and in which prefix they take. Matching rule shapes across
/// the verified units finds twenty-two productions with a `def` keyword; the other
/// fourteen end in a specialised body — `ActionBody`, `CaseBody`, `CalculationBody`,
/// `RequirementBody`, `StateDefBody`, `InterfaceBody`, `ViewDefinitionBody` — and they
/// name the declaration and the body separately rather than taking a `Definition`, so
/// none of them is here whether or not its body is implemented. `RequirementBody` is
/// implemented and `RequirementDefinition` still has a method of its own for exactly
/// that reason. `PortDefinition` does take a `Definition`, and is out for the opposite
/// reason: it carries a trailing `ConjugatedPortDefinitionMember` the eight do not.
#[derive(Clone, Copy)]
struct SimpleDefinition {
    /// The one keyword that says which production this is, before the `def`.
    keyword: &'static str,
    /// The node the production builds.
    node: SyntaxKind,
    /// Whether the prefix is an `OccurrenceDefinitionPrefix` rather than a
    /// `DefinitionPrefix`.
    ///
    /// The same distinction `SimpleUsage::is_occurrence` draws, one level up: only an
    /// occurrence may be `individual` (`SysML` 8.2.2.9.1). An attribute is not an
    /// occurrence, so `individual attribute def A;` is two errors rather than a prefix.
    is_occurrence: bool,
}

/// Every definition production sharing the `'def' Definition` spine.
///
/// The keywords are disjoint, so the order decides nothing.
const SIMPLE_DEFINITIONS: [SimpleDefinition; 8] = [
    SimpleDefinition {
        keyword: "attribute",
        node: SyntaxKind::AttributeDefinition,
        is_occurrence: false,
    },
    SimpleDefinition {
        keyword: "occurrence",
        node: SyntaxKind::OccurrenceDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "item",
        node: SyntaxKind::ItemDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "part",
        node: SyntaxKind::PartDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "connection",
        node: SyntaxKind::ConnectionDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "flow",
        node: SyntaxKind::FlowDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "allocation",
        node: SyntaxKind::AllocationDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "rendering",
        node: SyntaxKind::RenderingDefinition,
        is_occurrence: true,
    },
];

/// A `KerML` classifier production: one keyword over a shared spine.
///
/// Eight productions of `KerML` 8.2.4.2 are stated as `TypePrefix KEYWORD
/// ClassifierDeclaration TypeBody`, differing in the keyword and nothing else. The
/// derived units say so mechanically, so this table is a transcription rather than a
/// judgment, and writing eight near-identical methods would hide that they agree.
///
/// `Function` and `Predicate` share the shape but take a `FunctionBody`, and `Type`
/// takes a `TypeDeclaration` rather than a `ClassifierDeclaration`. None of those three
/// is implemented, and none is in this table, because the table is exactly the set whose
/// spine is shared.
#[derive(Clone, Copy)]
struct Classifier {
    /// The one keyword that says which production this is.
    keyword: &'static str,
    /// The node the production builds.
    node: SyntaxKind,
}

/// Every classifier production sharing the `ClassifierDeclaration TypeBody` spine.
///
/// The keywords are disjoint, so the order decides nothing.
const CLASSIFIERS: [Classifier; 8] = [
    Classifier {
        keyword: "classifier",
        node: SyntaxKind::Classifier,
    },
    Classifier {
        keyword: "class",
        node: SyntaxKind::Class,
    },
    Classifier {
        keyword: "struct",
        node: SyntaxKind::Structure,
    },
    Classifier {
        keyword: "datatype",
        node: SyntaxKind::DataType,
    },
    Classifier {
        keyword: "metaclass",
        node: SyntaxKind::Metaclass,
    },
    Classifier {
        keyword: "assoc",
        node: SyntaxKind::Association,
    },
    Classifier {
        keyword: "behavior",
        node: SyntaxKind::Behavior,
    },
    Classifier {
        keyword: "interaction",
        node: SyntaxKind::Interaction,
    },
];

struct Parser<'a> {
    source: &'a str,
    /// The grammar this text is read against, chosen once by the caller (ADR-0014).
    language: Language,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<Diagnostic>,
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
    /// annotation's body or an error, never text to skip past. Elsewhere this parser
    /// still attaches it as trivia, because the `AnnotatingMember` that would own it
    /// as an element in a package or definition body is not implemented.
    comments_significant: bool,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, language: Language) -> Self {
        Self {
            source,
            language,
            tokens: tokenize(source),
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            depth: 0,
            depth_reported: false,
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
    fn at_simple_definition(&self, n: usize) -> Option<SimpleDefinition> {
        SIMPLE_DEFINITIONS.iter().copied().find(|definition| {
            let after = if definition.is_occurrence {
                self.skip_occurrence_definition_prefix(n)
            } else {
                self.skip_definition_prefix(n)
            };
            self.nth_is_keyword(after, definition.keyword) && self.nth_is_keyword(after + 1, "def")
        })
    }

    /// The index just past a `DefinitionPrefix` written from the `n`th token.
    ///
    /// `DefinitionPrefix = BasicDefinitionPrefix? DefinitionExtensionKeyword*`
    /// (`SysML` 8.2.2.6.1). A `DefinitionExtensionKeyword` is not looked past: it is
    /// unimplemented, and leaving it to the enclosing body's recovery reports it.
    fn skip_definition_prefix(&self, n: usize) -> usize {
        n + usize::from(self.nth_is_keyword(n, "abstract") || self.nth_is_keyword(n, "variation"))
    }

    /// The index just past an `OccurrenceDefinitionPrefix` written from the `n`th token.
    ///
    /// A `DefinitionPrefix` and the one keyword only an occurrence may carry:
    /// `( 'individual' EmptyMultiplicityMember )?` (`SysML` 8.2.2.9.1). The same pair
    /// `skip_basic_usage_prefix` and `skip_occurrence_usage_prefix` make one level down.
    fn skip_occurrence_definition_prefix(&self, n: usize) -> usize {
        let n = self.skip_definition_prefix(n);
        n + usize::from(self.nth_is_keyword(n, "individual"))
    }

    /// Whether an implemented `DefinitionElement` starts at the `n`th meaningful token.
    fn at_definition_element(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "package")
            || self.at_port_definition(n)
            || self.at_requirement_definition(n)
            || self.at_constraint_definition(n)
            || self.at_calculation_definition(n)
            || self.at_action_definition(n)
            || self.at_simple_definition(n).is_some()
    }

    /// Whether an `ActionDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'action' 'def'` (`SysML` 8.2.2.17.1). Only the `def`
    /// separates it from an `ActionUsage`, which is unimplemented.
    fn at_action_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "action") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `ConstraintDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'constraint' 'def'` (`SysML` 8.2.2.20). Only the
    /// `def` separates it from a `ConstraintUsage`, which is unimplemented.
    fn at_constraint_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "constraint") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `CalculationDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'calc' 'def'` (`SysML` 8.2.2.19). Only the `def`
    /// separates it from a `CalculationUsage`, which is unimplemented and which the
    /// corpus writes.
    fn at_calculation_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "calc") && self.nth_is_keyword(after + 1, "def")
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
        let n = usize::from(self.at_visibility());
        if self.at_import()
            || self.at_element_keyword("alias")
            || self.at_annotating_member(n)
            // CalculationBodyItem = ActionBodyItem | ... (8.2.2.19), and ActionBodyItem
            // reads every usage a member does. Asked through the member's own recogniser
            // so the two cannot drift: kept as a separate list, it missed `action a;`
            // once and four productions after that.
            || self.at_sysml_keyword_member(n)
            // ActionBodyItem's control nodes, which no other body's member reaches.
            || self.at_control_node(n).is_some()
            // Asked only of a body that ends in a result expression, and
            // `ends_in_result_expression` says that is a calculation body alone.
            || self.at_source_succession_member(Body::Calculation)
        {
            return false;
        }
        if self.at_return_parameter_member() {
            // `return` continues the item run rather than ending it. The body loop asks
            // this question first and so never reaches here with a `return` in front of
            // it, but a recogniser that is only right because of where it is called is
            // the trap `membership`'s classifier guard is written against.
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

    /// Whether a `UsageCompletion` closes the construct that starts at the `n`th token.
    ///
    /// The bare-name collision, settled by looking for the thing a usage has and an
    /// expression has not. `Usage = UsageDeclaration UsageCompletion` and
    /// `UsageCompletion` ends in a body that is `';'` or a braced one (`SysML`
    /// 8.2.2.6.2), so a usage always reaches a `;` or a `{` before the enclosing body
    /// closes. An expression reaches the enclosing `}` instead.
    ///
    /// Brackets are counted so that a `;` or `{` inside a nested construct does not
    /// answer for this one. A `{` at depth zero says "usage" rather than opening a
    /// depth, because among what is implemented an expression never contains one —
    /// `BodyExpression` is unimplemented, and
    /// tests/rejection/body-expression-is-not-implemented.sysml holds that. When it
    /// lands, this is one of the places that has to change.
    fn usage_completion_follows(&self, n: usize) -> bool {
        let mut depth = 0u32;
        let mut i = n;
        while let Some(token) = self.peek_nth(i) {
            match token.kind {
                // A completion, so what starts at `n` is a usage and not an expression.
                SyntaxKind::Semicolon | SyntaxKind::LBrace if depth == 0 => return true,
                // The enclosing body closed and no completion was reached, so what is
                // here is the trailing expression.
                SyntaxKind::RBrace if depth == 0 => return false,
                // Guarded arms first, so these two only ever run nested.
                SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace => depth += 1,
                SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace => {
                    // Saturating because a stray closer is ordinary in an editor, and
                    // an underflow here would panic in debug (invariant 3).
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            i += 1;
        }
        // End of input with nothing closed. Truncated text is the editor's normal
        // state, and reading the remainder as an expression reports one error rather
        // than one per token.
        false
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

    /// Whether a `PortDefinition` starts at the `n`th meaningful token.
    ///
    /// Its own question rather than a row in `SIMPLE_DEFINITIONS`, because it is not on
    /// that spine: it takes a `DefinitionPrefix` like an attribute and then carries a
    /// trailing member none of the eight has.
    fn at_port_definition(&self, n: usize) -> bool {
        let after = self.skip_definition_prefix(n);
        self.nth_is_keyword(after, "port") && self.nth_is_keyword(after + 1, "def")
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
    /// Of `NonFeatureElement`, `Package` — a shared unit, the same production in both
    /// grammars — and the eight classifiers of 8.2.4.2 are implemented. Of
    /// `FeatureElement`'s ten alternatives, `Feature`, `BindingConnector` and `Succession`
    /// are. The rest (`step`, `connector`, `flow`, `succession flow`, …) are reported
    /// rather than read.
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
                self.nth_is_keyword(n, "package")
                    || self.at_classifier(n).is_some()
                    || self.at_feature(n)
                    || self.at_kerml_succession(n)
                    || self.at_kerml_binding_connector(n)
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
            || self.at_perform_action_usage(n)
            || self.at_flow_usage(n)
            || self.at_succession_as_usage(n)
            || self.at_binding_connector_as_usage(n)
            || self.at_assert_constraint_usage(n)
            || self.at_constraint_usage(n)
            || self.at_calculation_usage(n)
            || self.at_simple_usage(n).is_some()
            || self.at_reference_usage(n)
    }

    /// The index just past a `BasicUsagePrefix` written from the `n`th token.
    ///
    /// `BasicUsagePrefix = RefPrefix 'ref'?` over `RefPrefix = FeatureDirection?
    /// 'derived'? ( 'abstract' | 'variation' )? 'constant'?` (`SysML` 8.2.2.6.2).
    /// Every part is optional, so this returns `n` unchanged when none is written.
    ///
    /// The keywords are counted in the clause's order and each at most once, which is
    /// what makes `abstract in part p;` two errors rather than a longer prefix.
    ///
    /// This is also the whole of a `UsagePrefix` as implemented: `UsagePrefix =
    /// UnextendedUsagePrefix UsageExtensionKeyword*` and `UnextendedUsagePrefix =
    /// EndUsagePrefix | BasicUsagePrefix`, and neither `EndUsagePrefix` nor
    /// `UsageExtensionKeyword` is implemented, so neither is looked past — a usage
    /// carrying one is reported rather than silently accepted.
    fn skip_basic_usage_prefix(&self, n: usize) -> usize {
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract", "variation"],
            &["constant"],
            &["ref"],
        ] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
    }

    /// The index just past an `OccurrenceUsagePrefix` written from the `n`th token.
    ///
    /// `OccurrenceUsagePrefix = ( EndUsagePrefix | BasicUsagePrefix 'individual'?
    /// PortionKind? ) UsageExtensionKeyword*` (`SysML` 8.2.2.9.2) — a
    /// `BasicUsagePrefix` and the two keywords only an occurrence may carry.
    fn skip_occurrence_usage_prefix(&self, n: usize) -> usize {
        let mut n = self.skip_basic_usage_prefix(n);
        for words in [&["individual"][..], &["snapshot", "timeslice"]] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
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
            self.emit(
                DiagnosticCode::UnterminatedComment,
                Self::range_of(token),
                "comment is never closed: expected `*/`".to_owned(),
            );
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

    /// Report that the input is nested deeper than the parser will recurse.
    ///
    /// Said once rather than once per level: the limit is a property of the input as
    /// a whole, and one diagnostic per level would bury every other one.
    fn report_too_deep(&mut self) {
        if self.depth_reported {
            return;
        }
        self.depth_reported = true;
        let range = self.here();
        self.emit(
            DiagnosticCode::TooDeeplyNested,
            range,
            format!("nested deeper than {MAX_DEPTH} levels"),
        );
    }

    fn error_expected(&mut self, what: &str) {
        let found = self
            .peek()
            .map_or("end of file", |token| self.text_of(token));
        let message = format!("expected {what}, found `{found}`");
        let range = self.here();
        self.emit(DiagnosticCode::Expected, range, message);
    }

    /// Record one diagnostic.
    ///
    /// Every diagnostic this parser raises goes through here, so that the range is
    /// never forgotten: a `Diagnostic` without one cannot be constructed, which is what
    /// keeps "underline the offending token" from being a thing a caller has to guess.
    ///
    /// ONE DIAGNOSTIC PER START OFFSET. A repeat at the offset the last one starts at is
    /// dropped, keeping the FIRST raiser, which is the innermost: the production that
    /// could not read the token reports before the enclosing ones that then give up. Four
    /// reports about one `new` tell a reader nothing the first does not, and a token this
    /// parser cannot read is one defect however many nested productions fail on it.
    ///
    /// The range is compared, not the code: two codes at one offset (`PARSE-EXPECTED` and
    /// `PARSE-UNEXPECTED` both fire on an unreadable token) are the same defect described
    /// twice. Only the LAST diagnostic is compared, as Ruff's `add_error` does, because
    /// recovery moves forward: an offset that repeats does so consecutively, and comparing
    /// against every diagnostic would make this quadratic on a file full of errors.
    ///
    /// Nothing is lost that a caller needs: the tree still carries every byte
    /// (invariant 1), the element still enters the IR with its diagnostics (invariant 2,
    /// ADR-0002), and a defect at a NEW offset is never dropped.
    fn emit(&mut self, code: DiagnosticCode, range: TextRange, message: String) {
        if self
            .errors
            .last()
            .is_some_and(|last| last.range().start() == range.start())
        {
            return;
        }
        self.errors.push(Diagnostic::new(code, range, message));
    }

    /// The range a diagnostic about what comes next is about.
    ///
    /// The next non-trivia token, or an empty range at the end of the source when there
    /// is none. Empty is the honest answer for "expected `}`, found end of file": it is
    /// about a position rather than about any bytes, and it still has to be pointed at.
    fn here(&self) -> TextRange {
        self.peek().map_or_else(
            || TextRange::empty(Self::size(self.source.len())),
            Self::range_of,
        )
    }

    /// One token's half-open byte range.
    fn range_of(token: Token) -> TextRange {
        TextRange::new(Self::size(token.start), Self::size(token.end))
    }

    /// A byte offset as a `TextSize`, saturating rather than wrapping.
    ///
    /// `TextSize` is 32-bit. A source file large enough to overflow it is not one this
    /// parser has to place diagnostics in correctly, but it is one it must not panic on
    /// (invariant 3), so the conversion saturates and the position is merely wrong.
    fn size(offset: usize) -> TextSize {
        TextSize::new(u32::try_from(offset).unwrap_or(u32::MAX))
    }

    /// One token nothing accepts, wrapped so its bytes survive in the tree.
    fn error_token(&mut self) {
        self.eat_trivia();
        let Some(token) = self.tokens.get(self.pos).copied() else {
            return;
        };
        let message = format!("unexpected `{}`", self.text_of(token));
        self.emit(DiagnosticCode::Unexpected, Self::range_of(token), message);
        self.start_node(SyntaxKind::Error);
        self.push(token, token.kind);
        self.finish_node();
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
    fn root_namespace(mut self) -> (GreenNode, Vec<Diagnostic>) {
        self.start_node(SyntaxKind::RootNamespace);
        self.body_elements(None, Body::Root);
        // Trailing trivia belongs to the tree as much as anything else.
        self.eat_trivia();
        self.finish_node();
        (self.builder.finish(), self.errors)
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
        while !self.at_end() && !until.is_some_and(|kind| self.at(kind)) {
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
    fn body_element(&mut self, body: Body) -> bool {
        if self.depth >= MAX_DEPTH {
            // Too deeply nested to recurse into another body. Recover one token
            // at a time, exactly as unrecognised text is recovered: every byte
            // still reaches the tree, and the stack does not grow (invariant 3).
            self.report_too_deep();
            self.error_token();
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
            && (self.at_feature(usize::from(self.at_visibility()))
                || self.at_kerml_succession(usize::from(self.at_visibility()))
                || self.at_kerml_binding_connector(usize::from(self.at_visibility())))
        {
            // NamespaceMember = NonFeatureMember | NamespaceFeatureMember
            // (KerML 8.2.3.4.1). A feature is owned through the second, so it gets
            // its own membership node rather than the one `membership` builds.
            self.namespace_feature_member();
        } else if body.admits_action_body_item() && self.at_guarded_succession_member() {
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
            self.return_parameter_member();
        } else if body.ends_in_result_expression() && self.at_result_expression() {
            // The item run is over and what is left is the body's trailing
            // expression, which is not a member. `calculation_body_part` reads it;
            // the loop must not recover over it one token at a time.
            return false;
        } else if body.admits_requirement_constraint()
            && (self.at_element_keyword("require") || self.at_element_keyword("assume"))
        {
            // RequirementBodyItem's third alternative (SysML 8.2.2.21.1). Owns its
            // element through RequirementConstraintMembership, so like SubjectMember
            // below it cannot go through `membership`.
            self.requirement_constraint_member();
        } else if body.admits_subject() && self.at_element_keyword("subject") {
            // RequirementBodyItem's second alternative (SysML 8.2.2.21.1). Like
            // NamespaceFeatureMember above, it owns its element through a membership
            // of its own — SubjectMembership — so it cannot go through `membership`,
            // which builds the body's ordinary member node.
            self.subject_member();
        } else if body.admits_source_succession() && self.at_source_succession_member(body) {
            // `SourceSuccessionMember? <occurrence usage member>`, in whichever of
            // three item productions this body has; see `source_succession_item`.
            self.source_succession_item(body);
        } else if self.at_member_element(usize::from(self.at_visibility()))
            || (body.admits_action_body_item()
                && self
                    .at_control_node(usize::from(self.at_visibility()))
                    .is_some())
        {
            // A control node is an ActionNodeMember, ActionBehaviorMember's second
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

    /// Recover over text no item production accepts, as far as the statement goes.
    ///
    /// One `Error` node and ONE diagnostic for the whole run, where recovering a token at
    /// a time gave one of each per token: a line whose only defect was an unimplemented
    /// `new` reported 23 times, because each skipped token reported itself and each bare
    /// NAME in the middle of the run restarted a keywordless usage that then failed on the
    /// token after it.
    ///
    /// `SysML` has no newline terminator, so the boundary has to be written: a `;` at the
    /// depth recovery started from ends the statement and is taken; the enclosing `}` ends
    /// it and is LEFT for the body loop, which owns it; and a keyword that begins a member
    /// ends it too, so the valid item after a missing `;` is still read as an item rather
    /// than swallowed. A bare NAME does NOT end it, which is the whole point — that is the
    /// restart that cascaded.
    ///
    /// Braces are balanced while skipping, so a `;` or `}` inside a nested body does not
    /// end the outer statement. The first token is always taken, so recovery always
    /// advances and the loop that calls this cannot spin (invariant 3). Every token still
    /// enters the tree, so `parse(s).text() == s` holds over the skipped run as it did
    /// over the single tokens (invariant 1).
    fn recover_statement(&mut self) {
        self.eat_trivia();
        let Some(first) = self.tokens.get(self.pos).copied() else {
            return;
        };
        let start = Self::range_of(first).start();
        let mut end = Self::range_of(first).end();
        self.start_node(SyntaxKind::Error);
        let mut depth: usize = 0;
        let mut taken = false;
        while let Some(token) = self.tokens.get(self.pos).copied() {
            if taken && depth == 0 && (token.kind == SyntaxKind::RBrace || self.at_member_keyword())
            {
                break;
            }
            match token.kind {
                SyntaxKind::LBrace => depth += 1,
                SyntaxKind::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            if !self.skippable(token.kind) {
                end = Self::range_of(token).end();
                taken = true;
            }
            self.push(token, token.kind);
            if depth == 0 && token.kind == SyntaxKind::Semicolon {
                break;
            }
        }
        self.finish_node();
        let message = format!("unexpected `{}`", self.text_of(first));
        self.emit(
            DiagnosticCode::Unexpected,
            TextRange::new(start, end),
            message,
        );
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
                    self.at_keyword("package")
                        || self.at_classifier(0).is_some()
                        || self.at_kerml_succession(0)
                        || self.at_kerml_binding_connector(0)
                }
                Language::SysMl => {
                    self.at_definition_element(0)
                        || self.at_simple_usage(0).is_some()
                        || self.at_action_usage(0)
                        || self.at_perform_action_usage(0)
                        || self.at_flow_usage(0)
                        || self.at_succession_as_usage(0)
                        || self.at_binding_connector_as_usage(0)
                        || self.at_assert_constraint_usage(0)
                        || self.at_constraint_usage(0)
                        || self.at_calculation_usage(0)
                        || self.at_control_node(0).is_some()
                }
            }
    }

    /// A `SourceSuccessionMember` and the member it prefixes, as one item.
    ///
    /// `at_source_succession_member` has already seen an occurrence usage after the
    /// `then`, so `membership` builds the member the body's item production names for
    /// it: `OccurrenceUsageMember` in a definition body, `StructureUsageMember` or
    /// `BehaviorUsageMember` in an action body (8.2.2.6.1, 8.2.2.17.1).
    fn source_succession_item(&mut self, body: Body) {
        self.source_succession_member();
        let element = self.membership(body);
        self.behaviour_targets(body, element);
    }

    /// The `ActionTargetSuccessionMember*` after a behaviour usage in an action body.
    ///
    /// `ActionBodyItem`'s third alternative is `SourceSuccessionMember?
    /// ActionBehaviorMember ActionTargetSuccessionMember*` (8.2.2.17.1), so the `then X;`
    /// members belong to the item just read when — and only when — it was an
    /// `ActionBehaviorMember`, a `BehaviorUsageMember` or an `ActionNodeMember`; the
    /// `NonBehaviorBodyItem` alternative that reads structure usages takes no such
    /// suffix, so `part p; then b;` leaves the `then` reported.
    fn behaviour_targets(&mut self, body: Body, element: MemberElement) {
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
    // The two differ only in PackageMember's UsageElement alternative. A package owns a
    // usage through PackageMember itself; a definition or action body owns one through
    // a usage membership instead, never through DefinitionMember — see below. One method
    // builds all of them, the node chosen by `Body::member` from the body and the element.
    //
    // `DefinitionElement` and `UsageElement` get no node of their own. They are
    // alternations over element productions, and the element that matched already
    // says which alternative was taken, so a node here would add a level carrying
    // nothing. Neither is marked for coverage: `Package` and `PartDefinition` are
    // the only two of `DefinitionElement`'s 30 alternatives implemented, and the
    // seven in SIMPLE_USAGES are what is implemented of `UsageElement`'s.
    //
    // A definition is tried before a usage. The two share every prefix keyword and
    // the keyword after them, so `at_part_definition` — which requires the `def` —
    // must decide first; the usages are what is left.
    // Feature : Feature =
    //     ( FeaturePrefix ( 'feature' | ownedRelationship += PrefixMetadataMember )
    //       FeatureDeclaration?
    //     | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration
    //     ) ValuePart? TypeBody                                   (KerML 8.2.4.3.1)
    //
    // NOT marked for coverage. Both alternatives are read; the PrefixMetadataMember
    // alternative to the `feature` keyword is not. A feature may be introduced by a `#`
    // prefix instead of the word, and prefix metadata is unimplemented everywhere in
    // this parser.
    //
    // The two alternatives differ in exactly two things, and one method reads both:
    // whether the keyword is written, and whether the declaration is optional. It is
    // optional after the keyword and REQUIRED without one, because with no keyword the
    // declaration is the only thing that says a feature is here at all. That is why
    // `feature;` parses and `end;` does not.
    fn feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Feature);
        self.feature_prefix();
        if self.at_keyword("feature") {
            // The first alternative: the keyword carries it, so the declaration after
            // it is optional and `feature;` is a feature that declares nothing.
            self.bump_as(keyword("feature").unwrap_or(SyntaxKind::BasicName));
            if self.at_feature_declaration() {
                self.feature_declaration();
            }
        } else {
            // The second: no keyword at all, so the DECLARATION carries it and is
            // required. That asymmetry is the whole difference between the two, and it
            // is what makes `end;` an error while `end f;` is a feature.
            self.feature_declaration();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.type_body();
        self.finish_node();
    }

    /// Whether a `Feature` starts at the `n`th meaningful token, in either form.
    ///
    /// The keyword form is a prefix and then `feature`. The keywordless form is a prefix
    /// and then a declaration, which opens with `all`, a name, or a short name — and a
    /// keyword is not a name (`KerML` 8.2.2.6), so `package P;` and `class A;` are not
    /// mistaken for features that happen to be called `package` and `class`.
    ///
    /// The declaration's two bare forms, a `FeatureSpecializationPart` or a
    /// `ConjugationPart` with no name at all, are not recognised without a keyword:
    /// `: A;` alone at member position is left to recovery rather than read as an
    /// anonymous feature.
    fn at_feature(&self, n: usize) -> bool {
        let after = self.skip_feature_prefix(n);
        self.nth_is_keyword(after, "feature")
            || self.nth_is_keyword(after, "all")
            || self.nth_is_name(after)
            || self
                .peek_nth(after)
                .is_some_and(|t| t.kind == SyntaxKind::Lt)
    }

    /// Whether the `n`th meaningful token is a NAME.
    fn nth_is_name(&self, n: usize) -> bool {
        self.peek_nth(n).is_some_and(|token| self.is_name(token))
    }

    /// The index just past a `FeaturePrefix` written from the `n`th token.
    ///
    /// `FeaturePrefix = ( EndFeaturePrefix OwnedCrossFeatureMember? | BasicFeaturePrefix )
    /// PrefixMetadataMember*` (`KerML` 8.2.4.3.1). Neither `OwnedCrossFeatureMember` nor
    /// `PrefixMetadataMember` is looked past: both are unimplemented, and leaving them to
    /// the enclosing body's recovery reports them.
    fn skip_feature_prefix(&self, n: usize) -> usize {
        // EndFeaturePrefix = 'const'? 'end'. Tried first: it may open with `const`,
        // which is also BasicFeaturePrefix's last slot, and only the `end` tells them
        // apart.
        let end = n + usize::from(self.nth_is_keyword(n, "const"));
        if self.nth_is_keyword(end, "end") {
            return end + 1;
        }
        self.skip_basic_feature_prefix(n)
    }

    /// The index just past a `BasicFeaturePrefix` written from the `n`th token.
    ///
    /// Every part is optional, so this returns `n` unchanged when none is written. The
    /// keywords are counted in the clause's order and each at most once, which is what
    /// makes `derived in feature f;` two errors rather than a longer prefix.
    fn skip_basic_feature_prefix(&self, n: usize) -> usize {
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract"],
            &["composite", "portion"],
            &["var", "const"],
        ] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
    }

    // FeaturePrefix : Feature =
    //     ( EndFeaturePrefix ownedRelationship += OwnedCrossFeatureMember?
    //     | BasicFeaturePrefix ) ownedRelationship += PrefixMetadataMember*
    //                                                            (KerML 8.2.4.3.1)
    //
    // NOT marked: OwnedCrossFeatureMember and PrefixMetadataMember are unimplemented,
    // the same two gaps UsagePrefix has one level up. The node is built even when empty,
    // as MemberPrefix's is.
    fn feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeaturePrefix);
        if self.at_end_feature_prefix() {
            self.end_feature_prefix();
        } else {
            self.basic_feature_prefix();
        }
        self.finish_node();
    }

    /// Whether an `EndFeaturePrefix` rather than a `BasicFeaturePrefix` is written here.
    fn at_end_feature_prefix(&self) -> bool {
        self.nth_is_keyword(usize::from(self.at_keyword("const")), "end")
    }

    // production: EndFeaturePrefix
    //
    // EndFeaturePrefix : Feature = ( isConstant ?= 'const' )? isEnd ?= 'end'
    //                                                            (KerML 8.2.4.3.1)
    fn end_feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EndFeaturePrefix);
        self.eat_optional_keyword("const");
        self.expect_keyword("end");
        self.finish_node();
    }

    // production: BasicFeaturePrefix
    //
    // BasicFeaturePrefix : Feature =
    //     ( direction = FeatureDirection )? ( isDerived ?= 'derived' )?
    //     ( isAbstract ?= 'abstract' )?
    //     ( isComposite ?= 'composite' | isPortion ?= 'portion' )?
    //     ( isVariable ?= 'var' | isConstant ?= 'const' )?        (KerML 8.2.4.3.1)
    //
    // Every part is optional, so the node may be empty — `feature f;` writes a
    // FeaturePrefix whose BasicFeaturePrefix holds nothing.
    //
    // The two alternations forecloses: taking `composite` rules out `portion`, so
    // `composite portion feature f;` leaves the second word for the caller to report.
    fn basic_feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicFeaturePrefix);
        if self.at_keyword("in") || self.at_keyword("out") || self.at_keyword("inout") {
            self.feature_direction();
        }
        self.eat_optional_keyword("derived");
        self.eat_optional_keyword("abstract");
        self.eat_one_of(&["composite", "portion"]);
        self.eat_one_of(&["var", "const"]);
        self.finish_node();
    }

    /// Consume whichever of `words` is written here, or nothing.
    ///
    /// An alternation of keyword flags: taking one forecloses the others, which is what
    /// leaves the second word of `composite portion` for the caller to report.
    fn eat_one_of(&mut self, words: &[&str]) {
        if let Some(word) = words.iter().find(|word| self.at_keyword(word)) {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        }
    }

    /// Whether a `FeatureDeclaration` starts here.
    ///
    /// It is optional after the `feature` keyword, and it cannot be empty: a
    /// `FeatureIdentification` needs a name, and the other two alternatives need a
    /// specialization or a conjugation. So `feature;` is a feature with no declaration,
    /// and `feature : A;` is one whose declaration is a bare specialization.
    fn at_feature_declaration(&self) -> bool {
        self.at_keyword("all")
            || self.at_name()
            || self.at(SyntaxKind::Lt)
            || self.at_feature_specialization()
            || self.at_multiplicity_part()
    }

    // FeatureDeclaration : Feature =
    //     ( isSufficient ?= 'all' )?
    //     ( FeatureIdentification ( FeatureSpecializationPart | ConjugationPart )?
    //     | FeatureSpecializationPart
    //     | ConjugationPart )
    //     FeatureRelationshipPart*                                (KerML 8.2.4.3.1)
    //
    // NOT marked for coverage. ConjugationPart — the whole third alternative, and the
    // second half of the first — is unimplemented, as is FeatureRelationshipPart, which
    // reaches the chaining, inverting and featuring parts as well as the four
    // TypeRelationshipParts. Each is held by a case in tests/rejection/.
    fn feature_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureDeclaration);
        self.eat_optional_keyword("all");
        if self.at_name() || self.at(SyntaxKind::Lt) {
            self.feature_identification();
            if self.at_feature_specialization() || self.at_multiplicity_part() {
                self.feature_specialization_part();
            }
        } else {
            self.feature_specialization_part();
        }
        self.finish_node();
    }

    // production: FeatureIdentification
    //
    // FeatureIdentification : Feature =
    //     '<' declaredShortName = NAME '>' ( declaredName = NAME )?
    //   | declaredName = NAME                                     (KerML 8.2.4.3.1)
    //
    // NOT Identification, whose two parts are BOTH optional (SysML 8.2.2.2). A feature
    // declaration must name something, and that difference is the whole of what
    // tests/rejection/end-feature-requires-a-declaration.kerml records: the Pilot writes
    // Identification here and so accepts a declaration that names nothing.
    fn feature_identification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureIdentification);
        if self.at(SyntaxKind::Lt) {
            self.bump();
            self.expect_name("a short name");
            self.expect(SyntaxKind::Gt, "`>`");
            if self.at_name() {
                self.bump();
            }
        } else {
            self.expect_name("a feature name");
        }
        self.finish_node();
    }

    // production: NamespaceFeatureMember
    //
    // NamespaceFeatureMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += FeatureElement      (KerML 8.2.3.4.1)
    //
    // FeatureElement's ten alternatives are Feature, Step, Expression,
    // BooleanExpression, Invariant, Connector, BindingConnector, Succession, Flow and
    // SuccessionFlow. Three are implemented, Feature, BindingConnector and Succession; the
    // member itself is, which is what this marks, exactly as NonFeatureMember marks its own
    // shape rather than MemberElement's alternatives.
    fn namespace_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NamespaceFeatureMember);
        self.member_prefix();
        if self.at_kerml_succession(0) {
            self.kerml_succession();
        } else if self.at_kerml_binding_connector(0) {
            self.kerml_binding_connector();
        } else {
            self.feature();
        }
        self.finish_node();
    }

    /// Whether a `KerML` `Succession` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `succession`. The keyword is reserved (`KerML` 8.2.2.6),
    /// so it decides on its own, and `at_feature` never claims it as a name. The one
    /// other production that opens the same way is `SuccessionFlow`, `succession flow`
    /// (8.2.5.9.2): unimplemented, and declined here so that it is reported rather than
    /// read as a succession whose declaration names `flow`.
    fn at_kerml_succession(&self, n: usize) -> bool {
        let n = self.skip_feature_prefix(n);
        self.nth_is_keyword(n, "succession") && !self.nth_is_keyword(n + 1, "flow")
    }

    // production: Succession@kerml
    //
    // Succession : Succession =
    //     FeaturePrefix 'succession' SuccessionDeclaration TypeBody   (KerML 8.2.5.5.3)
    //
    // Scoped `kerml`: SysML writes its succession as SuccessionAsUsage (8.2.2.13.3), over
    // UsagePrefix and UsageDeclaration, and a .sysml file never reaches this (ADR-0014).
    // The two share ConnectorEndMember and nothing else, so the Rust name is scoped too.
    //
    // Marked although FeaturePrefix is not: this production's own four parts are read,
    // and FeaturePrefix's two gaps (OwnedCrossFeatureMember, PrefixMetadataMember) are
    // Feature's as much as this one's.
    //
    // implied specialization: Occurrences::happensBeforeLinks
    // constraint: Succession::checkSuccessionSpecialization
    //     `specializesFromLibrary('Occurrences::happensBeforeLinks')` (KerML 8.3.4.5.4).
    //     An injection, so sv2-hir's; this layer builds the tree only (ADR-0002).
    fn kerml_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Succession);
        self.feature_prefix();
        self.expect_keyword("succession");
        self.succession_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: SuccessionDeclaration@kerml
    //
    // SuccessionDeclaration : Succession =
    //     FeatureDeclaration
    //       ( 'first' ownedRelationship += ConnectorEndMember
    //         'then' ownedRelationship += ConnectorEndMember )?
    //   | ( isSufficient ?= 'all' )?
    //       ( 'first'? ownedRelationship += ConnectorEndMember
    //         'then' ownedRelationship += ConnectorEndMember )?      (KerML 8.2.5.5.3)
    //
    // The clause writes `s.isSufficient`; the stray `s.` names a property and consumes
    // no token, so the rule is the Xtext fragment's (recorded on the unit).
    //
    // The two alternatives are told apart before either commits, on what follows an
    // optional `all`. The second is taken when a `first` is written there, when nothing
    // is (the empty declaration: `succession;`, `succession { }`), or when a ConnectorEnd
    // is written there and a `then` directly after it (`succession a then b;`). Anything
    // else is a FeatureDeclaration: `succession s first a then b;`, `succession : T;`.
    // The cases are disjoint — `first` and `then` are reserved, so no FeatureDeclaration
    // opens on `first`, and a FeatureDeclaration followed by `then` is no succession.
    // `all` is left to FeatureDeclaration's own slot in the first alternative.
    //
    // The node is built even when empty, as FeaturePrefix's is.
    fn succession_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuccessionDeclaration);
        let after_all = usize::from(self.at_keyword("all"));
        let empty = self.nth_is(after_all, SyntaxKind::Semicolon)
            || self.nth_is(after_all, SyntaxKind::LBrace)
            || self.peek_nth(after_all).is_none();
        let ends = self.nth_is_keyword(after_all, "first")
            || self
                .skip_connector_end(after_all)
                .is_some_and(|after| self.nth_is_keyword(after, "then"));
        if empty || ends {
            self.eat_optional_keyword("all");
            if ends {
                self.eat_optional_keyword("first");
                self.connector_end_member();
                self.expect_keyword("then");
                self.connector_end_member();
            }
        } else {
            self.feature_declaration();
            if self.at_keyword("first") {
                self.bump_as(keyword("first").unwrap_or(SyntaxKind::BasicName));
                self.connector_end_member();
                self.expect_keyword("then");
                self.connector_end_member();
            }
        }
        self.finish_node();
    }

    /// Whether a `KerML` `BindingConnector` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `binding`. The keyword is reserved (`KerML` 8.2.2.6) and
    /// opens no other `KerML` production, so it decides on its own, and `at_feature` never
    /// claims it as a name. The prefix skipped is `FeaturePrefix`, which is what
    /// `kerml_binding_connector` reads: a recogniser that looked past more than its
    /// production consumes would make the body loop ask again at the same token.
    fn at_kerml_binding_connector(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "binding")
    }

    // production: BindingConnector@kerml
    //
    // BindingConnector : BindingConnector =
    //     FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody   (KerML 8.2.5.5.2)
    //
    // Scoped `kerml`, as Succession is: SysML writes its binding as
    // BindingConnectorAsUsage (8.2.2.13.2), over UsagePrefix and `bind`, and a .sysml file
    // never reaches this (ADR-0014). The two share ConnectorEndMember and nothing else,
    // so the Rust name is scoped too. Receipt 02578996 is the production's.
    //
    // Marked although FeaturePrefix is not, for the reason Succession gives.
    //
    // implied specialization: Links::selfLinks
    // constraint: BindingConnector::checkBindingConnectorSpecialization
    //     `specializesFromLibrary('Links::selfLinks')` (KerML 8.3.4.5.2), which "requires
    //     that BindingConnectors specialize the Feature Links::selfLinks" (8.4.4.6.2,
    //     receipt c87b1db0). An injection, so sv2-hir's; this layer builds the tree only
    //     (ADR-0002).
    // constraint: BindingConnector::validateBindingConnectorIsBinary
    //     `relatedFeature->size() = 2` (KerML 8.3.4.5.2). NOT held by this grammar, unlike
    //     SysML's: the declaration's ends are optional, so `binding;` and a binding whose
    //     ends are declared as end features in its body both parse, and whether there are
    //     two is a question for the layer that counts relatedFeatures (ADR-0002: validity
    //     gates writes, never reads).
    fn kerml_binding_connector(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnector);
        self.feature_prefix();
        self.expect_keyword("binding");
        self.binding_connector_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: BindingConnectorDeclaration@kerml
    //
    // BindingConnectorDeclaration : BindingConnector =
    //     FeatureDeclaration
    //       ( 'of' ownedRelationship += ConnectorEndMember
    //         '=' ownedRelationship += ConnectorEndMember )?
    //   | ( isSufficient ?= 'all' )?
    //       ( 'of'? ownedRelationship += ConnectorEndMember
    //         '=' ownedRelationship += ConnectorEndMember )?          (KerML 8.2.5.5.2)
    //
    // SuccessionDeclaration's shape, `of` for `first` and `=` for `then`, and told apart
    // the same way on what follows an optional `all`: the second alternative when `of` is
    // written there, when nothing is (`binding;`, `binding { ... }`), or when a
    // ConnectorEnd is and `=` directly after it (`binding a = b;`); a FeatureDeclaration
    // otherwise (`binding ab of a = b;`, `binding : T;`). Disjoint because `of` is
    // reserved (8.2.2.6), so no FeatureDeclaration opens on it, and a FeatureDeclaration
    // takes no `=` here — this production writes no ValuePart. "If a binding connector
    // declaration includes only the related features part, then the keyword of can be
    // omitted" (7.4.6.3, receipt cda2047f).
    //
    // The node is built even when empty, as SuccessionDeclaration's is.
    fn binding_connector_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnectorDeclaration);
        let after_all = usize::from(self.at_keyword("all"));
        let empty = self.nth_is(after_all, SyntaxKind::Semicolon)
            || self.nth_is(after_all, SyntaxKind::LBrace)
            || self.peek_nth(after_all).is_none();
        let ends = self.nth_is_keyword(after_all, "of")
            || self
                .skip_connector_end(after_all)
                .is_some_and(|after| self.nth_is(after, SyntaxKind::Eq));
        if empty || ends {
            self.eat_optional_keyword("all");
            if ends {
                self.eat_optional_keyword("of");
                self.connector_end_member();
                self.expect(SyntaxKind::Eq, "`=`");
                self.connector_end_member();
            }
        } else {
            self.feature_declaration();
            if self.at_keyword("of") {
                self.bump_as(keyword("of").unwrap_or(SyntaxKind::BasicName));
                self.connector_end_member();
                self.expect(SyntaxKind::Eq, "`=`");
                self.connector_end_member();
            }
        }
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
    // NamespaceFeatureMember, its sibling, is NOT implemented: every one of
    // FeatureElement's ten alternatives is unimplemented, so there is nothing to own.
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
    // the node. Each is marked as the member it is, as DefinitionMember is: the
    // <kind>UsageElement alternations are not, most of their alternatives being
    // unimplemented. BehaviorUsageMember is reached only from ActionBodyItem's third
    // alternative, whose `then` prefix and trailing target successions its callers read.
    //
    // production: ActionNodeMember@sysml
    //
    // ActionNodeMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += ActionNode            (SysML 8.2.2.17.1)
    //
    // The same shape once more, marked as the member it is while ActionNode is not: of
    // its eight alternatives only ControlNode is read, and SendNode, AcceptNode,
    // AssignmentNode, TerminateNode, IfNode, WhileLoopNode and ForLoopNode are reported.
    //
    // production: ActionBehaviorMember@sysml
    //
    // ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember   (SysML 8.2.2.17.1)
    //
    // An alternation with no node, marked because both of its alternatives are read —
    // the convention OwnedExpression follows. Which one was taken is the member node.
    fn membership(&mut self, body: Body) -> MemberElement {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.member_prefix();
        let mut element = MemberElement::Other;
        if self.at_annotating_member(0) {
            // The body is a REGULAR_COMMENT, so it has to be a token here rather than
            // trivia for the production to be able to read it.
            self.with_significant_comments(Self::annotating_element);
        } else if self.at_keyword("package") {
            self.package();
        } else if let Some(classifier) = self.at_classifier(0).filter(|_| {
            // Every classifier unit is scoped `kerml`. SysML reaches DefinitionElement
            // instead, so `class Foo;` in a .sysml file is text SysML does not state.
            // The guard is here rather than left to `at_member_element`'s caller,
            // because a dispatch that is only correct when reached one way is a trap.
            self.language == Language::KerMl
        }) {
            self.classifier(classifier);
        } else if self.language == Language::KerMl {
            self.error_expected("a package or a classifier");
        } else if let Some((word, node)) = self
            .at_control_node(0)
            .filter(|_| body.admits_action_body_item())
        {
            // Only the action-body family reaches ActionNodeMember; `body_elements`
            // asks the same question before calling here. The guard is repeated for
            // the reason the classifier one above is: a dispatch that is only correct
            // when reached one way is a trap.
            self.control_node(node, word);
            element = MemberElement::ActionNode;
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

    /// `SysML`'s `DefinitionElement`, minus the `package` `membership` takes first.
    /// Returns whether one was read.
    ///
    /// `DefinitionElement` gets no node, as `UsageElement` gets none: it is an
    /// alternation, and the alternative that matched says which was taken
    /// (`SysML` 8.2.2.5.2, Package Elements — where BOTH alternations are stated, not
    /// 8.2.2.6.1, which is Definitions and only USES `DefinitionElement` inside
    /// `DefinitionMember`). Written in the same order as `at_definition_element`, its
    /// recogniser, so that the two cannot silently disagree about what a member may be.
    ///
    /// Order is not load-bearing here — each alternative is introduced by its own
    /// keyword pair, and a keyword is not a name (`SysML` 8.2.2.1.2). It is in
    /// `usage_element`, which says why there.
    fn definition_element(&mut self) -> bool {
        if self.at_port_definition(0) {
            self.port_definition();
        } else if self.at_requirement_definition(0) {
            self.requirement_definition();
        } else if self.at_constraint_definition(0) {
            self.constraint_definition();
        } else if self.at_calculation_definition(0) {
            self.calculation_definition();
        } else if self.at_action_definition(0) {
            self.action_definition();
        } else if let Some(definition) = self.at_simple_definition(0) {
            self.simple_definition(definition);
        } else {
            return false;
        }
        true
    }

    /// `SysML`'s `UsageElement`. Returns whether one was read.
    ///
    /// `UsageElement` is stated at `SysML` 8.2.2.5.2 beside `DefinitionElement`. It is
    /// reachable from `PackageMember` (8.2.2.5.1) and not from `NamespaceMember`
    /// (`KerML` 8.2.3.4.1), which is why the caller asks this only for `SysML` — `KerML`
    /// has no usages at all. The three clauses are named separately because they are
    /// three different facts; an earlier revision cited 8.2.2.6.1 for all of it.
    ///
    /// ORDER IS LOAD-BEARING, and each step of it is a defect that was fixed once:
    ///
    /// - the keyword usages come before the two reference usages, because
    ///   `at_reference_usage` sees the `ref` in `ref attribute y;` too, and that `ref`
    ///   is an `AttributeUsage`'s `BasicUsagePrefix` rather than a `ReferenceUsage`;
    /// - `DefaultReferenceUsage` is last of all, because it is the usage with no
    ///   keyword, so everything that opens with one has already been taken.
    fn usage_element(&mut self) -> bool {
        self.usage_element_of_class().is_some()
    }

    /// `usage_element`, answering which of 8.2.2.6.4's classes the usage read is in.
    ///
    /// `ActionUsage` and `PerformActionUsage` are `BehaviorUsageElement`s, `FlowUsage` a
    /// `StructureUsageElement`;
    /// `SuccessionAsUsage`, `BindingConnectorAsUsage`, `ReferenceUsage` and
    /// `DefaultReferenceUsage` are `NonOccurrenceUsageElement`s; the seven `SIMPLE_USAGES`
    /// carry their own.
    fn usage_element_of_class(&mut self) -> Option<UsageClass> {
        if self.at_perform_action_usage(0) {
            self.perform_action_usage();
            Some(UsageClass::Behavior)
        } else if self.at_action_usage(0) {
            self.action_usage();
            Some(UsageClass::Behavior)
        } else if self.at_calculation_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ActionUsage is.
            self.calculation_usage();
            Some(UsageClass::Behavior)
        } else if self.at_constraint_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as AssertConstraintUsage is.
            self.constraint_usage();
            Some(UsageClass::Behavior)
        } else if self.at_assert_constraint_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ActionUsage is.
            self.assert_constraint_usage();
            Some(UsageClass::Behavior)
        } else if self.at_flow_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), though its metaclass is an ActionUsage.
            self.flow_usage();
            Some(UsageClass::Structure)
        } else if self.at_succession_as_usage(0) {
            // A NonOccurrenceUsageElement (8.2.2.6.4): "a succession is not a kind of
            // occurrence usage" (7.13.5, receipt 2abd302c).
            self.succession_as_usage();
            Some(UsageClass::NonOccurrence)
        } else if self.at_binding_connector_as_usage(0) {
            // A NonOccurrenceUsageElement (8.2.2.6.4): "a binding is not a kind of
            // occurrence usage" (7.13.3, receipt 6db87b41).
            self.binding_connector_as_usage();
            Some(UsageClass::NonOccurrence)
        } else if let Some(usage) = self.at_simple_usage(0) {
            self.simple_usage(usage);
            Some(usage.class)
        } else if self.at_reference_usage(0) {
            self.reference_usage();
            Some(UsageClass::NonOccurrence)
        } else if self.at_default_reference_usage(0) {
            self.default_reference_usage();
            Some(UsageClass::NonOccurrence)
        } else {
            None
        }
    }

    // ReferenceUsage : ReferenceUsage =
    //     ( EndUsagePrefix | RefPrefix ) 'ref' Usage             (SysML 8.2.2.6.2)
    //
    // NOT marked for coverage: EndUsagePrefix, the first of the two alternatives, is
    // unimplemented — the same gap OccurrenceUsagePrefix has, held by
    // tests/rejection/end-usage-prefix-is-not-implemented.sysml.
    //
    // The `ref` here is the production's own keyword, not BasicUsagePrefix's optional
    // one. `ref attribute y;` is an AttributeUsage whose prefix carries `ref`, and
    // `ref y;` is a ReferenceUsage; the difference is whether a usage keyword follows,
    // which is why `at_simple_usage` is asked first.
    fn reference_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ReferenceUsage);
        self.ref_prefix();
        self.expect_keyword("ref");
        self.usage();
        self.finish_node();
    }

    /// Whether a `ReferenceUsage` starts at the `n`th meaningful token.
    fn at_reference_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_ref_prefix(n), "ref")
    }

    /// The index just past a `RefPrefix` written from the `n`th token.
    ///
    /// `RefPrefix = FeatureDirection? 'derived'? ( 'abstract' | 'variation' )?
    /// 'constant'?` (`SysML` 8.2.2.6.2) — every part optional, and NOT including the
    /// `ref` that `BasicUsagePrefix` adds after it.
    fn skip_ref_prefix(&self, n: usize) -> usize {
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract", "variation"],
            &["constant"],
        ] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
    }

    // production: DefaultReferenceUsage
    //
    // DefaultReferenceUsage : ReferenceUsage =
    //     ( isEnd ?= 'end' )? RefPrefix
    //     ( Identification FeatureSpecializationPart? | FeatureSpecializationPart )
    //     UsageCompletion                                        (SysML 8.2.2.6.2)
    //
    // A usage with NO keyword, carried by its declaration alone — SysML's analogue of
    // KerML's keywordless Feature, and the same shape: a name with an optional
    // specialization, or a bare specialization with no name. The second form is what
    // `:>> length = 4800 [mm];` is, a redefinition that names nothing.
    //
    // Unlike KerML's Feature, the bare-specialization form IS read here, because this
    // production states it directly rather than reaching it through a FeatureDeclaration
    // shared with a keyword form.
    fn default_reference_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefaultReferenceUsage);
        self.eat_optional_keyword("end");
        self.ref_prefix();
        if self.at_name() || self.at(SyntaxKind::Lt) {
            self.identification();
            if self.at_feature_specialization() || self.at_multiplicity_part() {
                self.feature_specialization_part();
            }
        } else {
            self.feature_specialization_part();
        }
        self.usage_completion();
        self.finish_node();
    }

    /// Whether a `DefaultReferenceUsage` starts at the `n`th meaningful token.
    ///
    /// Asked last of the usages, because it is the one with no keyword: anything that
    /// opens with `part`, `attribute`, `ref` or a definition keyword has already been
    /// taken by then, and a reserved keyword is not a name (`SysML` 8.2.2.1.2), so
    /// `package P;` is not read as a usage called `package`.
    fn at_default_reference_usage(&self, n: usize) -> bool {
        let after = self.skip_ref_prefix(n + usize::from(self.nth_is_keyword(n, "end")));
        self.nth_is_name(after)
            || self
                .peek_nth(after)
                .is_some_and(|t| t.kind == SyntaxKind::Lt)
            || self.nth_at_feature_specialization(after)
    }

    // production: AttributeUsage
    // production: EnumerationUsage
    // production: ItemUsage
    // production: OccurrenceUsage
    // production: PartUsage
    // production: PortUsage
    // production: RenderingUsage
    //
    // AttributeUsage   = UsagePrefix           'attribute'  Usage  (SysML 8.2.2.7)
    // EnumerationUsage = UsagePrefix           'enum'       Usage  (SysML 8.2.2.8)
    // ItemUsage        = OccurrenceUsagePrefix 'item'       Usage  (SysML 8.2.2.10)
    // OccurrenceUsage  = OccurrenceUsagePrefix 'occurrence' Usage  (SysML 8.2.2.9.2)
    // PartUsage        = OccurrenceUsagePrefix 'part'       Usage  (SysML 8.2.2.11)
    // PortUsage        = OccurrenceUsagePrefix 'port'       Usage  (SysML 8.2.2.12)
    // RenderingUsage   = OccurrenceUsagePrefix 'rendering'  Usage  (SysML 8.2.2.26.3)
    //
    // Seven productions, one method, as PackageMember and DefinitionMember share
    // `membership`. They differ in exactly two things — the keyword, and whether the
    // prefix is an OccurrenceUsagePrefix or a UsagePrefix — so SIMPLE_USAGES carries
    // those two and nothing else. Writing seven near-identical methods would not make
    // any of them more faithful to its clause; it would make a difference between
    // them harder to see.
    //
    // Each is marked separately because each IS fully implemented. What none of them
    // implements lives below, in the prefixes and in FeatureSpecializationPart, and
    // is recorded there.
    fn simple_usage(&mut self, usage: SimpleUsage) {
        self.eat_trivia();
        self.start_node(usage.node);
        if usage.is_occurrence {
            self.occurrence_usage_prefix();
        } else {
            self.usage_prefix();
        }
        self.expect_keyword(usage.keyword);
        self.usage();
        self.finish_node();
    }

    /// Which of `SIMPLE_USAGES` starts at the `n`th meaningful token, if any.
    ///
    /// The keyword decides, and the `def` after it rules a usage out: every one of
    /// these has a definition counterpart spelled the same way but for that word
    /// (`SysML` 8.2.2.6.1), and none of those definitions is implemented.
    /// Whether an `ActionUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'action'` with no `def` after it (`SysML` 8.2.2.17.2). The
    /// `def` is what separates it from an `ActionDefinition`, exactly as it separates
    /// each of `SIMPLE_USAGES` from the definition spelled the same way.
    fn at_action_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "action") && !self.nth_is_keyword(after + 1, "def")
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

    /// Whether a `PerformActionUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'perform'` (`SysML` 8.2.2.17.2). No `def` test, because
    /// there is no `perform def`: the keyword names a usage and nothing else.
    fn at_perform_action_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "perform")
    }

    fn at_simple_usage(&self, n: usize) -> Option<SimpleUsage> {
        SIMPLE_USAGES.iter().copied().find(|usage| {
            let after = if usage.is_occurrence {
                self.skip_occurrence_usage_prefix(n)
            } else {
                self.skip_basic_usage_prefix(n)
            };
            self.nth_is_keyword(after, usage.keyword) && !self.nth_is_keyword(after + 1, "def")
        })
    }

    // UsagePrefix : Usage = UnextendedUsagePrefix UsageExtensionKeyword*
    //                                                            (SysML 8.2.2.6.2)
    //
    // UnextendedUsagePrefix = EndUsagePrefix | BasicUsagePrefix
    //
    // NOT marked for coverage, and neither is UnextendedUsagePrefix. EndUsagePrefix,
    // one of the two alternatives, is unimplemented, and so is UsageExtensionKeyword
    // (`#` prefix metadata) — the same two gaps OccurrenceUsagePrefix has, and held
    // by the same rejection case.
    //
    // UnextendedUsagePrefix gets no node: it is an alternation, and the alternative
    // that matched says which was taken.
    //
    // The node is built even when empty, as MemberPrefix's is.
    fn usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsagePrefix);
        if self.at_basic_usage_prefix() {
            self.basic_usage_prefix();
        }
        self.finish_node();
    }

    // OccurrenceUsagePrefix : OccurrenceUsage =
    //     ( EndUsagePrefix
    //     | BasicUsagePrefix ( isIndividual ?= 'individual' )?
    //       ( portionKind = PortionKind )?
    //     ) UsageExtensionKeyword*                               (SysML 8.2.2.9.2)
    //
    // NOT marked for coverage. Two parts are unimplemented, and each is a construct
    // the language has rather than an optional slot left empty:
    //
    //   - EndUsagePrefix (`'end' OwnedCrossFeatureMember?`), the whole first
    //     alternative. The corpus writes no `end part`, but the clause admits it, and
    //     tests/rejection/end-usage-prefix-is-not-implemented.sysml holds the absence.
    //   - UsageExtensionKeyword (`#` prefix metadata, a PrefixMetadataMember), as on
    //     OccurrenceDefinitionPrefix. `at_part_usage` does not look past a `#`, so a
    //     usage carrying one never reaches here.
    //
    // The node is built even when every slot is empty, as MemberPrefix's is.
    fn occurrence_usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OccurrenceUsagePrefix);
        if self.at_basic_usage_prefix() {
            self.basic_usage_prefix();
        }
        if self.at_keyword("individual") {
            self.bump_as(keyword("individual").unwrap_or(SyntaxKind::BasicName));
        }
        if self.at_keyword("snapshot") || self.at_keyword("timeslice") {
            self.portion_kind();
        }
        self.finish_node();
    }

    /// Whether any keyword of a `BasicUsagePrefix` is written here.
    fn at_basic_usage_prefix(&self) -> bool {
        [
            "in",
            "out",
            "inout",
            "derived",
            "abstract",
            "variation",
            "constant",
            "ref",
        ]
        .iter()
        .any(|word| self.at_keyword(word))
    }

    // production: BasicUsagePrefix
    //
    // BasicUsagePrefix : Usage = RefPrefix ( isReference ?= 'ref' )?
    //                                                            (SysML 8.2.2.6.2)
    fn basic_usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicUsagePrefix);
        self.ref_prefix();
        if self.at_keyword("ref") {
            self.bump_as(keyword("ref").unwrap_or(SyntaxKind::BasicName));
        }
        self.finish_node();
    }

    // production: RefPrefix
    //
    // RefPrefix : Usage = ( direction = FeatureDirection )?
    //     ( isDerived ?= 'derived' )?
    //     ( isAbstract ?= 'abstract' | isVariation ?= 'variation' )?
    //     ( isConstant ?= 'constant' )?                          (SysML 8.2.2.6.2)
    //
    // Every part is optional, so the node may be empty — `ref part p;` writes a
    // BasicUsagePrefix whose RefPrefix holds nothing.
    fn ref_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RefPrefix);
        if self.at_keyword("in") || self.at_keyword("out") || self.at_keyword("inout") {
            self.feature_direction();
        }
        self.eat_optional_keyword("derived");
        // `isAbstract ?= 'abstract' | isVariation ?= 'variation'` is an alternation,
        // so taking one forecloses the other: `abstract variation part p;` leaves
        // `variation` for the caller to report rather than consuming both.
        if let Some(word) = ["abstract", "variation"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        }
        self.eat_optional_keyword("constant");
        self.finish_node();
    }

    /// Consume `text` if it is written here, leaving the position alone if it is not.
    fn eat_optional_keyword(&mut self, text: &str) {
        if self.at_keyword(text) {
            self.bump_as(keyword(text).unwrap_or(SyntaxKind::BasicName));
        }
    }

    // production: FeatureDirection
    //
    // FeatureDirection : FeatureDirectionKind = 'in' | 'out' | 'inout'
    //                                                            (SysML 8.2.2.6.2)
    fn feature_direction(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureDirection);
        match ["in", "out", "inout"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`in`, `out` or `inout`"),
        }
        self.finish_node();
    }

    // production: PortionKind
    //
    // PortionKind = 'snapshot' | 'timeslice'                     (SysML 8.2.2.9.2)
    fn portion_kind(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PortionKind);
        match ["snapshot", "timeslice"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`snapshot` or `timeslice`"),
        }
        self.finish_node();
    }

    // production: Usage
    //
    // Usage = UsageDeclaration UsageCompletion                   (SysML 8.2.2.6.2)
    fn usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Usage);
        self.usage_declaration();
        self.usage_completion();
        self.finish_node();
    }

    // production: UsageDeclaration
    //
    // UsageDeclaration : Usage = Identification FeatureSpecializationPart?
    //                                                            (SysML 8.2.2.6.2)
    fn usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageDeclaration);
        self.identification();
        if self.at_feature_specialization() || self.at_multiplicity_part() {
            self.feature_specialization_part();
        }
        self.finish_node();
    }

    /// Whether a `FeatureSpecialization` is written here.
    ///
    /// `FeatureSpecialization = Typings | Subsettings | References | Crosses
    /// | Redefinitions` (`SysML` 8.2.2.6.5). Each opens with one of the special
    /// lexical terminals of 8.2.2.1.2, which has a symbol form and a word form, and
    /// the five are told apart on that one token.
    ///
    /// The symbols are checked before the words because the lexer produces a distinct
    /// kind for each symbol, while every word arrives as a `BasicName`.
    fn at_feature_specialization(&self) -> bool {
        self.nth_at_feature_specialization(0)
    }

    /// `at_feature_specialization`, asked of the `n`th meaningful token.
    ///
    /// Needed where the specialization is not next: a `DefaultReferenceUsage` may open
    /// on one after its prefix, and a `PayloadFeature` decides its alternative on one
    /// after a name and a multiplicity. There was a second copy of this test, for the
    /// first of those, that spelled only `SysML`'s `defined by`; one method now answers
    /// for both positions and both languages.
    fn nth_at_feature_specialization(&self, n: usize) -> bool {
        const SYMBOLS: [SyntaxKind; 5] = [
            SyntaxKind::Colon,        // DEFINED_BY
            SyntaxKind::ColonGt,      // SUBSETS
            SyntaxKind::ColonGtGt,    // REDEFINES
            SyntaxKind::ColonColonGt, // REFERENCES
            SyntaxKind::FatArrow,     // CROSSES
        ];
        SYMBOLS.iter().any(|kind| self.nth_is(n, *kind))
            || ["subsets", "redefines", "references", "crosses"]
                .iter()
                .any(|word| self.nth_is_keyword(n, word))
            || (self.nth_is_keyword(
                n,
                match self.language {
                    Language::KerMl => "typed",
                    Language::SysMl => "defined",
                },
            ) && self.nth_is_keyword(n + 1, "by"))
    }

    // FeatureSpecialization = Typings | Subsettings | References | Crosses
    //                       | Redefinitions                      (SysML 8.2.2.6.5)
    //
    // No node of its own, as DefinitionElement and UsageElement have none: it is an
    // alternation, and the alternative that matched already says which was taken, so
    // a node here would add a level carrying nothing. It is not marked for coverage
    // for the same reason — there is no method that is it.
    fn feature_specialization(&mut self) {
        if self.at(SyntaxKind::ColonGt) || self.at_keyword("subsets") {
            self.subsettings();
        } else if self.at(SyntaxKind::ColonGtGt) || self.at_keyword("redefines") {
            self.redefinitions();
        } else if self.at(SyntaxKind::ColonColonGt) || self.at_keyword("references") {
            self.references();
        } else if self.at(SyntaxKind::FatArrow) || self.at_keyword("crosses") {
            self.crosses();
        } else {
            self.typings();
        }
    }

    // production: Subsettings
    //
    // Subsettings : Feature = Subsets ( ',' ownedRelationship += OwnedSubsetting )*
    //                                                            (SysML 8.2.2.6.5)
    fn subsettings(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subsettings);
        self.subsets();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_subsetting();
        }
        self.finish_node();
    }

    // production: Subsets
    //
    // Subsets : Feature = SUBSETS ownedRelationship += OwnedSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // SUBSETS = ':>' | 'subsets'                                 (SysML 8.2.2.1.2)
    fn subsets(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subsets);
        self.terminal(SyntaxKind::ColonGt, "subsets", "`:>` or `subsets`");
        self.owned_subsetting();
        self.finish_node();
    }

    // production: OwnedSubsetting@sysml
    //
    // OwnedSubsetting : Subsetting =
    //     subsettedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    fn owned_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedSubsetting);
    }

    // production: Redefinitions
    //
    // Redefinitions : Feature =
    //     Redefines ( ',' ownedRelationship += OwnedRedefinition )*
    //                                                            (SysML 8.2.2.6.5)
    fn redefinitions(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Redefinitions);
        self.redefines();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_redefinition();
        }
        self.finish_node();
    }

    // production: Redefines
    //
    // Redefines : Feature = REDEFINES ownedRelationship += OwnedRedefinition
    //                                                            (SysML 8.2.2.6.5)
    //
    // REDEFINES = ':>>' | 'redefines'                            (SysML 8.2.2.1.2)
    fn redefines(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Redefines);
        self.terminal(SyntaxKind::ColonGtGt, "redefines", "`:>>` or `redefines`");
        self.owned_redefinition();
        self.finish_node();
    }

    // production: OwnedRedefinition@sysml
    //
    // OwnedRedefinition : Redefinition =
    //     redefinedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    fn owned_redefinition(&mut self) {
        self.chainable_target(SyntaxKind::OwnedRedefinition);
    }

    // production: References
    //
    // References : Feature =
    //     REFERENCES ownedRelationship += OwnedReferenceSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // REFERENCES = '::>' | 'references'                          (SysML 8.2.2.1.2)
    //
    // One target, with no repetition — unlike Subsettings and Redefinitions beside
    // it, so a ',' after the target belongs to whatever encloses this.
    fn references(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::References);
        self.terminal(
            SyntaxKind::ColonColonGt,
            "references",
            "`::>` or `references`",
        );
        self.owned_reference_subsetting();
        self.finish_node();
    }

    // production: OwnedReferenceSubsetting@sysml
    //
    // OwnedReferenceSubsetting : ReferenceSubsetting =
    //     referencedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // The second alternative was MARKED AND ABSENT until this change: the marker claimed
    // the production and the body read only a QualifiedName, so `part p :>> a.b;` was
    // rejected while coverage counted the production as done. The corpus writes a chained
    // reference 60 times. A false `implemented` is the one kind of coverage error that
    // cannot be found by reading the report, which is why it survived.
    fn owned_reference_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedReferenceSubsetting);
    }

    /// One of the five chainable targets of `SysML` 8.2.2.6.5, under `node`.
    ///
    /// `OwnedFeatureTyping`, `OwnedSubsetting`, `OwnedRedefinition`,
    /// `OwnedReferenceSubsetting` and `OwnedCrossSubsetting` are stated with one shape and
    /// differ only in which feature the target is assigned to:
    ///
    /// ```text
    /// <x>Feature = [QualifiedName] | <x>Feature = OwnedFeatureChain
    /// ```
    ///
    /// All five were MARKED with the chain alternative absent, so `part p :>> a.b;` and
    /// `attribute x : a.b;` were rejected while coverage counted five productions as done.
    /// One method now reads the shape they share, which is also what keeps them from
    /// drifting apart again — four were fixed together and the fifth was missed precisely
    /// because it was a separate copy of the same three lines.
    fn chainable_target(&mut self, node: SyntaxKind) {
        self.eat_trivia();
        self.start_node(node);
        let start = self.builder.checkpoint();
        self.qualified_name();
        if self.at_feature_chain() {
            self.owned_feature_chain(start);
        }
        self.finish_node();
    }

    // production: OwnedFeatureChain@sysml
    //
    // OwnedFeatureChain : Feature =
    //     ownedRelationship += OwnedFeatureChaining
    //     ( '.' ownedRelationship += OwnedFeatureChaining )+     (SysML 8.2.2.6.5)
    //
    // production: OwnedFeatureChaining
    //
    // OwnedFeatureChaining : FeatureChaining =
    //     chainingFeature = [QualifiedName]                      (SysML 8.2.2.6.5)
    //
    // NOT the expression layer's chain. `a.b` after `:>>` is this; `a.b` in an expression
    // is a FeatureChainExpression (KerML 8.2.5.8.2). The two tokens are identical and the
    // POSITION decides, exactly as it does for `[` between a MultiplicityRange and a
    // BracketExpression — and for the same structural reason: this one is reachable only
    // from a reference, and that one only from a primary operand, and neither position
    // can be reached from the other.
    //
    // The shapes differ too, which is why one could not serve for both. This is FLAT —
    // `( '.' link )+` over one node, with every link a sibling — where the expression
    // chain FOLDS, nesting one FeatureChainExpression per link. The `+` means a chain has
    // at least two links, so a bare `a` is the QualifiedName alternative and never an
    // OwnedFeatureChain of one.
    //
    // The first link is already in the tree when the `.` is seen, so the node is opened
    // retroactively at `start`, as the postfix expressions do.
    fn owned_feature_chain(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::OwnedFeatureChain);
        self.wrap_at(start, &[SyntaxKind::OwnedFeatureChaining]);
        while self.at_feature_chain() {
            self.bump();
            self.owned_feature_chaining();
        }
        self.finish_node();
    }

    fn owned_feature_chaining(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedFeatureChaining);
        self.qualified_name();
        self.finish_node();
    }

    // production: Crosses
    //
    // Crosses : Feature = CROSSES ownedRelationship += OwnedCrossSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // CROSSES = '=>' | 'crosses'                                 (SysML 8.2.2.1.2)
    //
    // One target, as References has.
    fn crosses(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Crosses);
        self.terminal(SyntaxKind::FatArrow, "crosses", "`=>` or `crosses`");
        self.owned_cross_subsetting();
        self.finish_node();
    }

    // production: OwnedCrossSubsetting@sysml
    //
    // OwnedCrossSubsetting : CrossSubsetting =
    //     crossedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    fn owned_cross_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedCrossSubsetting);
    }

    /// Consume one of the special lexical terminals of `SysML` 8.2.2.1.2, in either
    /// spelling: the operator `symbol`, or `word` written out.
    fn terminal(&mut self, symbol: SyntaxKind, word: &str, what: &str) {
        if self.at(symbol) {
            self.bump();
        } else if self.at_keyword(word) {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        } else {
            self.error_expected(what);
        }
    }

    // FeatureSpecializationPart : Feature =
    //     FeatureSpecialization+ MultiplicityPart? FeatureSpecialization*
    //     | MultiplicityPart FeatureSpecialization*              (KerML 8.2.4.3.1)
    //
    // FeatureSpecialization = Typings | Subsettings | References | Crosses
    //                       | Redefinitions                      (SysML 8.2.2.6.5)
    //
    // production: FeatureSpecializationPart
    //
    // The two alternatives differ only in where the MultiplicityPart may sit: the
    // first requires at least one FeatureSpecialization before it, the second puts it
    // first, and both allow FeatureSpecializations after it. Their union is therefore
    // any number of FeatureSpecializations with AT MOST ONE MultiplicityPart anywhere
    // among them, which is what the loop below reads — and neither alternative admits
    // an empty part, which is why `usage_declaration` asks before entering.
    //
    // A second MultiplicityPart is what the one-shot flag rejects; neither
    // alternative can produce one, and
    // tests/rejection/multiplicity-part-appears-once.sysml holds that.
    //
    // The production above is the clause's, verbatim. The Pilot writes the first
    // alternative as `( -> FeatureSpecialization )+` (SysML.xtext:366); that `->` is a
    // syntactic predicate, an LL workaround for its parser generator, and it appears
    // in NEITHER specification's BNF (KerML-textual-bnf.kebnf:632,
    // SysML-textual-bnf.kebnf:440). It is not ported, and it is not quoted here as
    // though the clause contained it — deviations.json makes Tier B `never_used_for`
    // rule bodies, and a predicate copied into a citation is exactly that.
    fn feature_specialization_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureSpecializationPart);
        let mut multiplicity_taken = false;
        loop {
            if self.at_feature_specialization() {
                self.feature_specialization();
            } else if !multiplicity_taken && self.at_multiplicity_part() {
                self.multiplicity_part();
                multiplicity_taken = true;
            } else {
                break;
            }
        }
        self.finish_node();
    }

    // production: Typings@sysml
    //
    // Typings : Feature = TypedBy ( ',' ownedRelationship += FeatureTyping )*
    //                                                            (SysML 8.2.2.6.5)
    fn typings(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Typings);
        self.typed_by();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.feature_typing();
        }
        self.finish_node();
    }

    // production: TypedBy@sysml
    //
    // TypedBy : Feature = DEFINED_BY ownedRelationship += FeatureTyping
    //                                                            (SysML 8.2.2.6.5)
    //
    // DEFINED_BY = ':' | 'defined' 'by'                          (SysML 8.2.2.1.2)
    //
    // SysML reserves `defined`, not `typed`: KerML's TypedBy spells the same position
    // `':' | 'typed' 'by'`, and deviations.json records that split under
    // MetadataUsageDeclaration.
    fn typed_by(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypedBy);
        if self.at(SyntaxKind::Colon) {
            self.bump();
        } else {
            // The one place the two grammars spell this differently:
            // TYPED_BY = ':' | 'typed' 'by'    (KerML 8.2.4.3.1)
            // DEFINED_BY = ':' | 'defined' 'by' (SysML 8.2.2.1.2)
            // The `:` form is shared, which is what the corpus almost always writes.
            self.expect_keyword(match self.language {
                Language::KerMl => "typed",
                Language::SysMl => "defined",
            });
            self.expect_keyword("by");
        }
        self.feature_typing();
        self.finish_node();
    }

    // production: FeatureTyping@sysml
    //
    // FeatureTyping = OwnedFeatureTyping | ConjugatedPortTyping  (SysML 8.2.2.6.5)
    //
    // ConjugatedPortTyping (`~` a port definition) is not implemented; a usage typed
    // by one is reported rather than accepted.
    fn feature_typing(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureTyping);
        self.owned_feature_typing();
        self.finish_node();
    }

    // production: OwnedFeatureTyping@sysml
    //
    // OwnedFeatureTyping : FeatureTyping =
    //     type = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    //                                                            (SysML 8.2.2.6.5)
    //
    // OwnedFeatureChain needs two or more segments joined by '.', and a FeatureChain has
    // at least two by construction, so a bare QualifiedName is never ambiguous with one.
    //
    // This was the FIFTH production marked with the chain alternative absent, and the one
    // the earlier sweep missed: the other four are subsettings and redefinitions, and this
    // is a TYPING, so a search for reference productions did not reach it. Its own comment
    // said the alternative was not implemented and the marker claimed it anyway. Every
    // production in the grammar that admits an OwnedFeatureChain has now been checked; see
    // the [coverage-marker-audit] pending decision for the method.
    fn owned_feature_typing(&mut self) {
        self.chainable_target(SyntaxKind::OwnedFeatureTyping);
    }

    // production: UsageCompletion
    //
    // UsageCompletion : Usage = ValuePart? UsageBody             (SysML 8.2.2.6.2)
    fn usage_completion(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageCompletion);
        if self.at_value_part() {
            self.value_part();
        }
        self.usage_body();
        self.finish_node();
    }

    /// Whether a `ValuePart` is written here (`SysML` 8.2.2.6.2).
    ///
    /// `FeatureValue` opens with `'='`, `':='` or `'default'`, and nothing else a
    /// `UsageCompletion` may hold opens with any of them.
    fn at_value_part(&self) -> bool {
        self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) || self.at_keyword("default")
    }

    // production: ValuePart
    //
    // ValuePart : Usage = ownedRelationship += FeatureValue      (SysML 8.2.2.6.2)
    fn value_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ValuePart);
        self.feature_value();
        self.finish_node();
    }

    // production: FeatureValue
    //
    // FeatureValue : FeatureValue =
    //     ( '=' | isInitial ?= ':='
    //     | isDefault ?= 'default' ( '=' | isInitial ?= ':=' )? )
    //     ownedRelatedElement += OwnedExpression                 (SysML 8.2.2.6.2)
    //
    // The three prefixes are three flags on one relationship, not three productions:
    // `=` binds, `:=` initialises, and `default` marks either as a default. After
    // `default` the `=` is optional, so `default x` and `default = x` are the same
    // FeatureValue with isDefault set.
    fn feature_value(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureValue);
        if self.at_keyword("default") {
            self.bump_as(SyntaxKind::KwDefault);
            if self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) {
                self.bump();
            }
        } else if self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) {
            self.bump();
        } else {
            self.error_expected("`=`, `:=` or `default`");
        }
        self.owned_expression();
        self.finish_node();
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

    // -- multiplicity, SysML 8.2.2.6.6 ------------------------------------------

    /// Whether a `MultiplicityPart` is written here (`SysML` 8.2.2.6.6).
    ///
    /// Either the `'['` of its `OwnedMultiplicity` or one of the two keywords its
    /// second alternative may carry without one.
    fn at_multiplicity_part(&self) -> bool {
        self.at(SyntaxKind::LBracket) || self.at_keyword("ordered") || self.at_keyword("nonunique")
    }

    // production: MultiplicityPart
    //
    // MultiplicityPart : Feature =
    //       ownedRelationship += OwnedMultiplicity
    //     | ( ownedRelationship += OwnedMultiplicity )?
    //       ( isOrdered ?= 'ordered' ( { isUnique = false } 'nonunique' )?
    //       | { isUnique = false } 'nonunique' ( isOrdered ?= 'ordered' )? )
    //                                                            (SysML 8.2.2.6.6)
    //
    // The two alternatives together admit an OwnedMultiplicity, the two keywords in
    // either order, or both — and the first alternative is what makes the keywords
    // optional. `nonunique` sets isUnique false in both orderings, which is why the
    // keyword appears twice in the clause and once here.
    fn multiplicity_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityPart);
        if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
        }
        if self.at_keyword("ordered") {
            self.bump_as(SyntaxKind::KwOrdered);
            if self.at_keyword("nonunique") {
                self.bump_as(SyntaxKind::KwNonunique);
            }
        } else if self.at_keyword("nonunique") {
            self.bump_as(SyntaxKind::KwNonunique);
            if self.at_keyword("ordered") {
                self.bump_as(SyntaxKind::KwOrdered);
            }
        }
        self.finish_node();
    }

    // production: OwnedMultiplicity@sysml
    //
    // OwnedMultiplicity : OwningMembership =
    //     ownedRelatedElement += MultiplicityRange               (SysML 8.2.2.6.6)
    //
    // SysML's OwnedMultiplicity owns a MultiplicityRange directly. KerML writes
    // `ownedRelatedElement += OwnedMultiplicityRange` over its own MultiplicityRange,
    // which is the named `'multiplicity' Identification MultiplicityBounds TypeBody`
    // element — a different production with the same name, split by scope in
    // ADR-0015. This is the SysML reading.
    fn owned_multiplicity(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedMultiplicity);
        self.multiplicity_range();
        self.finish_node();
    }

    // production: MultiplicityRange@sysml
    //
    // MultiplicityRange : MultiplicityRange =
    //     '[' ( ownedRelationship += MultiplicityExpressionMember '..' )?
    //           ownedRelationship += MultiplicityExpressionMember ']'
    //                                                            (SysML 8.2.2.6.6)
    //
    // The lower bound is the optional one, so `[2]` is an upper bound alone and
    // `[0..*]` is both. Read left to right: one member, then the second only if a
    // `'..'` separates them.
    fn multiplicity_range(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityRange);
        self.expect(SyntaxKind::LBracket, "`[`");
        self.multiplicity_expression_member();
        if self.at(SyntaxKind::DotDot) {
            self.bump();
            self.multiplicity_expression_member();
        }
        self.expect(SyntaxKind::RBracket, "`]`");
        self.finish_node();
    }

    // production: MultiplicityExpressionMember
    //
    // MultiplicityExpressionMember : OwningMembership =
    //     ownedRelatedElement += ( LiteralExpression
    //                            | FeatureReferenceExpression )  (SysML 8.2.2.6.6)
    //
    // A bound is a literal or a name and NOT an OwnedExpression: the clause names
    // two alternatives, neither of which is an operator expression. `[1+1]` is
    // therefore not a multiplicity, which is what keeps this production cheap and
    // what tests/rejection/multiplicity-bound-is-not-an-operator-expression.sysml
    // holds.
    fn multiplicity_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityExpressionMember);
        if self.at_literal_expression() {
            self.literal_expression();
        } else if self.at_feature_reference() {
            self.feature_reference_expression();
        } else {
            self.error_expected("a literal or a name");
        }
        self.finish_node();
    }

    // -- the expression layer, KerML 8.2.5.8 -------------------------------------

    // production: OwnedExpression
    //
    // OwnedExpression : Expression =
    //       ConditionalExpression
    //     | ConditionalBinaryOperatorExpression
    //     | BinaryOperatorExpression
    //     | UnaryOperatorExpression
    //     | ClassificationExpression
    //     | MetaclassificationExpression
    //     | ExtentExpression
    //     | PrimaryExpression                                    (KerML 8.2.5.8.1)
    //
    // THE ALTERNATION CARRIES NO PRECEDENCE, AND CANNOT. KerML 8.2.5.8.1 note 2
    // states that the grouping of nested OperatorExpressions is not expressed in the
    // productions above it, and is instead determined by the precedence of the
    // operators as given in that clause's table 6. So this production is ambiguous
    // on its own, by the specification's own construction, and any recursive-descent
    // parser has to carry the table separately.
    //
    // deviations.json records the decision under BinaryOperatorExpression
    // (spec_only/follow_spec): implement the specification's one-production shape and
    // supply precedence from the table, rather than porting the Pilot's stratified
    // cascade. The cascade is docs/DERIVATION.md's named example of an Xtext LL
    // workaround; porting it would report coverage against a dozen productions the
    // language does not have, and would move every precedence question from the
    // table, where the specification answers it, to rule ordering.
    //
    // The table is data at docs/operator-precedence.toml, cited to the clause.
    // `INFIX` and `UNARY_OPERATORS` below are that file's tiers; the test
    // `the_infix_table_is_the_recorded_precedence_table` reads the file and holds the
    // two against each other, so an edit to either alone fails.
    fn owned_expression(&mut self) {
        self.expression(TIER_LOOSEST);
    }

    /// An `OwnedExpression` whose top-level operator binds no looser than `tier`.
    ///
    /// This is the precedence climb. `tier` is the loosest tier this call may
    /// consume: a caller that has just taken a tier-5 operator asks for tier 4 on
    /// the right, and the `+` in `a + b + c` is therefore left for the caller's own
    /// loop rather than nested, which is what makes the tier group to the left.
    fn expression(&mut self, tier: u8) {
        self.eat_trivia();
        if self.depth >= MAX_DEPTH {
            self.report_too_deep();
            return;
        }
        self.depth += 1;
        self.expression_inner(tier);
        self.depth -= 1;
    }

    /// [`Parser::expression`] proper, entered one nesting level deeper.
    fn expression_inner(&mut self, tier: u8) {
        // ConditionalExpression is tier 15, the loosest, and is written prefix, so
        // it is decided before anything else rather than found by the climb.
        if tier >= TIER_CONDITIONAL && self.at_keyword("if") {
            self.conditional_expression();
            return;
        }
        let start = self.builder.checkpoint();
        self.prefix_expression();
        self.infix_tail(start, tier);
    }

    /// The `OwnedExpression` alternatives written with their operator first, and the
    /// `PrimaryExpression` left when none of them is.
    fn prefix_expression(&mut self) {
        // ExtentExpression = 'all' TypeReferenceMember — tier 1, tighter than every
        // binary operator.
        if self.at_keyword("all") {
            self.extent_expression();
            return;
        }
        // MetaclassificationExpression's left operand is a MetadataArgumentMember,
        // which reaches a QualifiedName and not an OwnedExpression, so it cannot be
        // wrapped retroactively around an already-parsed expression the way the other
        // tier-8 operators are. It is decided here instead, by looking past the name.
        if self.at_metaclassification() {
            self.metaclassification_expression();
            return;
        }
        // ClassificationExpression's ArgumentMember is optional, so a classification
        // operator may open one with nothing to its left. `filter @Safety;` is the
        // corpus idiom for exactly that.
        if self.at_leading_classification() {
            self.classification_expression(None);
            return;
        }
        // UnaryOperatorExpression = UnaryOperator ArgumentMember EmptyResultMember —
        // tier 2, which binds TIGHTER than exponentiation at tier 3, so `-2 ** 2` is
        // `(-2) ** 2`. That is the opposite of the C and Python convention and is
        // what table 6 states.
        if self.at_unary_operator() {
            self.unary_operator_expression();
            return;
        }
        self.primary_expression();
    }

    // production: BinaryOperatorExpression
    //
    // BinaryOperatorExpression : OperatorExpression =
    //     ownedRelationship += ArgumentMember
    //     operator = BinaryOperator
    //     ownedRelationship += ArgumentMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // production: ConditionalBinaryOperatorExpression
    //
    // ConditionalBinaryOperatorExpression : OperatorExpression =
    //     ownedRelationship += ArgumentMember
    //     operator = ConditionalBinaryOperator
    //     ownedRelationship += ArgumentExpressionMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // Both are built here rather than in a method each, because they differ only in
    // the membership of their right operand and in which operators name them — and
    // both of those are columns of the table. ClassificationExpression is reached
    // from here too, but has its own method because its left operand is optional.
    //
    /// Consume infix operators at `max_tier` or tighter, folding the expression that
    /// starts at `start` into each one's left operand.
    ///
    /// The left operand is already in the tree when the operator is read, so the
    /// operator's node and the memberships around its left operand are both opened
    /// retroactively at `start` — the same technique `import_declaration` uses for
    /// two alternatives that share a prefix.
    fn infix_tail(&mut self, start: rowan::Checkpoint, max_tier: u8) {
        while let Some(op) = self.infix_operator_here() {
            if op.tier > max_tier {
                break;
            }
            self.start_node_at(start, op.node);
            self.wrap_at(start, op.left);
            self.bump_spelling(op.spelling);
            self.right_operand(op);
            // Every OperatorExpression the table reaches owns a result parameter,
            // written nowhere in the text. ExtentExpression is the one that does not,
            // and it is prefix, so it is not in the table.
            self.empty_result_member();
            self.finish_node();
        }
    }

    /// The right operand of an infix operator, in the membership the clause names.
    ///
    /// The tier asked for is what makes the group: strictly tighter for a
    /// left-associative operator, so a repeat is left to the caller's loop, and the
    /// same tier for a right-associative one, so a repeat nests here.
    fn right_operand(&mut self, op: &InfixOperator) {
        let tier = match op.assoc {
            Assoc::Left => op.tier.saturating_sub(1),
            Assoc::Right => op.tier,
        };
        match op.right {
            Operand::Argument => self.argument_member(tier),
            Operand::ArgumentExpression => self.argument_expression_member(tier),
            Operand::TypeReference => self.type_reference_member(SyntaxKind::TypeReferenceMember),
            Operand::TypeResult => self.type_reference_member(SyntaxKind::TypeResultMember),
        }
    }

    // production: ConditionalExpression
    //
    // ConditionalExpression : OperatorExpression =
    //     operator = 'if'
    //     ownedRelationship += ArgumentMember '?'
    //     ownedRelationship += ArgumentExpressionMember 'else'
    //     ownedRelationship += ArgumentExpressionMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // Tier 15, the loosest. Both branches are ArgumentExpressionMembers and the
    // condition is an ArgumentMember: the branches are referenced rather than
    // evaluated, which is how the abstract syntax reifies the short circuit. The
    // `else` branch is read at the same tier, so `if a? b else if c? d else e`
    // nests to the right without parentheses; the condition is read one tier tighter,
    // so a bare `if` there is not.
    fn conditional_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConditionalExpression);
        self.expect_keyword("if");
        self.argument_member(TIER_CONDITIONAL - 1);
        self.expect(SyntaxKind::Question, "`?`");
        self.argument_expression_member(TIER_CONDITIONAL);
        self.expect_keyword("else");
        self.argument_expression_member(TIER_CONDITIONAL);
        self.empty_result_member();
        self.finish_node();
    }

    // production: UnaryOperatorExpression
    //
    // UnaryOperatorExpression : OperatorExpression =
    //     operator = UnaryOperator
    //     ownedRelationship += ArgumentMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // UnaryOperator = '+' | '-' | '~' | 'not'                    (KerML 8.2.5.8.1)
    //
    // The operand is read at tier 2, which is what makes `- -a` nest — the
    // specification's ArgumentMember reaches OwnedExpression, so a unary operand may
    // itself be unary. The Pilot's UnaryExpression rule does not recurse and rejects
    // it; that is a property of its parser generator, not of the language, and it is
    // not ported.
    fn unary_operator_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UnaryOperatorExpression);
        self.bump_unary_operator();
        self.argument_member(TIER_UNARY);
        self.empty_result_member();
        self.finish_node();
    }

    // production: ExtentExpression
    //
    // ExtentExpression : OperatorExpression =
    //     operator = 'all'
    //     ownedRelationship += TypeReferenceMember               (KerML 8.2.5.8.1)
    //
    // Tier 1, the tightest, and the one OperatorExpression in the clause that owns no
    // EmptyResultMember — its result comes from the type, so none is written here.
    fn extent_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExtentExpression);
        self.expect_keyword("all");
        self.type_reference_member(SyntaxKind::TypeReferenceMember);
        self.finish_node();
    }

    // production: ClassificationExpression
    //
    // ClassificationExpression : OperatorExpression =
    //     ( ownedRelationship += ArgumentMember )?
    //     ( operator = ClassificationTestOperator
    //       ownedRelationship += TypeReferenceMember
    //     | operator = CastOperator
    //       ownedRelationship += TypeResultMember )
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // ClassificationTestOperator = 'istype' | 'hastype' | '@'    (KerML 8.2.5.8.1)
    // CastOperator              = 'as'                           (KerML 8.2.5.8.1)
    //
    // The ArgumentMember is optional, so this is reached two ways: from `infix_tail`
    // with a left operand already in the tree, and from `prefix_expression` with
    // none. `start` is the checkpoint of the left operand, or `None` when there is
    // no left operand to fold in.
    //
    // The right operand is a type and not an expression, in two different
    // memberships: a test names a TypeReferenceMember and a cast a TypeResultMember.
    fn classification_expression(&mut self, start: Option<rowan::Checkpoint>) {
        self.eat_trivia();
        match start {
            Some(start) => {
                self.start_node_at(start, SyntaxKind::ClassificationExpression);
                self.wrap_at(start, LEFT_ARGUMENT);
            }
            None => self.start_node(SyntaxKind::ClassificationExpression),
        }
        let is_cast = self.at_keyword("as");
        self.bump_classification_operator();
        let member = if is_cast {
            SyntaxKind::TypeResultMember
        } else {
            SyntaxKind::TypeReferenceMember
        };
        self.type_reference_member(member);
        self.empty_result_member();
        self.finish_node();
    }

    // production: MetaclassificationExpression
    //
    // MetaclassificationExpression : OperatorExpression =
    //     ownedRelationship += MetadataArgumentMember
    //     ( operator = MetaclassificationTestOperator
    //       ownedRelationship += TypeReferenceMember
    //     | operator = MetaCastOperator
    //       ownedRelationship += TypeResultMember )
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // MetaclassificationTestOperator = '@@'                      (KerML 8.2.5.8.1)
    // MetaCastOperator               = 'meta'                    (KerML 8.2.5.8.1)
    //
    // Tier 8 with the classification operators, but NOT interchangeable with them:
    // its left operand is a MetadataArgumentMember, and that reaches a QualifiedName
    // (MetadataArgument -> MetadataValue -> MetadataReference ->
    // ElementReferenceMember = [QualifiedName]) rather than an OwnedExpression. So
    // the left operand of `meta` is a reference, never an expression, and
    // `(a + b) meta T` is not something the clause can express. That also makes it
    // unchainable: `x meta A meta B` would need a MetaclassificationExpression where
    // a QualifiedName is required, so it is reported.
    fn metaclassification_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetaclassificationExpression);
        self.metadata_argument_member();
        let is_cast = self.at_keyword("meta");
        if is_cast {
            self.bump_as(SyntaxKind::KwMeta);
        } else {
            self.expect(SyntaxKind::AtAt, "`@@`");
        }
        let member = if is_cast {
            SyntaxKind::TypeResultMember
        } else {
            SyntaxKind::TypeReferenceMember
        };
        self.type_reference_member(member);
        self.empty_result_member();
        self.finish_node();
    }

    // production: MetadataArgumentMember
    //
    // MetadataArgumentMember : ParameterMembership =
    //     ownedRelatedElement += MetadataArgument                (KerML 8.2.5.8.1)
    //
    // production: MetadataArgument
    //
    // MetadataArgument : Feature =
    //     ownedRelationship += MetadataValue                     (KerML 8.2.5.8.1)
    //
    // production: MetadataValue
    //
    // MetadataValue : FeatureValue = value = MetadataReference   (KerML 8.2.5.8.1)
    //
    // production: MetadataReference
    //
    // MetadataReference : MetadataAccessExpression =
    //     ownedRelationship += ElementReferenceMember            (KerML 8.2.5.8.1)
    //
    // production: ElementReferenceMember
    //
    // ElementReferenceMember : Membership =
    //     memberElement = [QualifiedName]                        (KerML 8.2.5.8.3)
    //
    // Five memberships over one name. They are nodes rather than collapsed because
    // the abstract syntax reifies each of them, and sv2-hir will need to walk them.
    fn metadata_argument_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataArgumentMember);
        self.start_node(SyntaxKind::MetadataArgument);
        self.start_node(SyntaxKind::MetadataValue);
        self.start_node(SyntaxKind::MetadataReference);
        self.start_node(SyntaxKind::ElementReferenceMember);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // PrimaryExpression = FeatureChainExpression
    //                   | NonFeatureChainPrimaryExpression       (KerML 8.2.5.8.2)
    //
    // NonFeatureChainPrimaryExpression = BracketExpression | IndexExpression
    //     | SequenceExpression | SelectExpression | CollectExpression
    //     | FunctionOperationExpression | BaseExpression         (KerML 8.2.5.8.2)
    //
    // BaseExpression = NullExpression | LiteralExpression
    //     | FeatureReferenceExpression | MetadataAccessExpression
    //     | InvocationExpression | ConstructorExpression
    //     | BodyExpression                                       (KerML 8.2.5.8.3)
    //
    // NONE OF THE THREE IS MARKED FOR COVERAGE. Each is an alternation and each has
    // alternatives that are absent, so marking any of them would claim a production
    // this parser does not read. What is implemented is SequenceExpression from the
    // middle one, and NullExpression, LiteralExpression and FeatureReferenceExpression
    // from the last. What is not, each with a rejection case naming the clause:
    //
    //   FeatureChainExpression     `a.b`            the postfix `.`
    //   BracketExpression          `1200 [kg]`      the quantity form of `[`
    //   IndexExpression            `tanks#(1)`
    //   SelectExpression           `x.?{ ... }`     needs BodyExpression
    //   CollectExpression          `x.{ ... }`      needs BodyExpression
    //   FunctionOperationExpression `x->size()`
    //   MetadataAccessExpression   `E.metadata`
    //   InvocationExpression       `f(1, 2)`        needs ArgumentList
    //   ConstructorExpression      `new T(1)`       needs ArgumentList
    //   BodyExpression             `{ in x; x }`    reaches ExpressionBody, and in
    //                                              SysML that reads CalculationBody,
    //                                              which is most of the language
    //
    // No node of its own for any of the three, as DefinitionElement and
    // FeatureSpecialization have none: an alternation's node would add a level
    // carrying nothing, because the alternative that matched already says which was
    // taken.
    fn primary_expression(&mut self) {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.non_feature_chain_primary_expression();
        self.postfix_tail(start);
    }

    // production: FeatureChainExpression
    //
    // FeatureChainExpression =
    //     ownedRelationship += NonFeatureChainPrimaryArgumentMember '.'
    //     ownedRelationship += FeatureChainMember                 (KerML 8.2.5.8.2)
    //
    // production: NonFeatureChainPrimaryArgumentMember
    //
    // NonFeatureChainPrimaryArgumentMember =
    //     ownedMemberParameter = PrimaryArgument                  (KerML 8.2.5.8.2)
    //
    // production: PrimaryArgument
    // production: PrimaryArgumentValue
    //
    // PrimaryArgument      = ownedRelationship += PrimaryArgumentValue
    // PrimaryArgumentValue = value = PrimaryExpression            (KerML 8.2.5.8.2)
    //
    // FOLDS LEFT, so `a.b.c` is `(a.b).c`. The member is named
    // NonFeatureChainPrimaryArgumentMember and its body is `PrimaryArgument`, NOT
    // NonFeatureChainPrimaryArgument — both the clause and Tier B' state it that way,
    // and the derived unit adjudicates it: the Pilot builds the same association with
    // `{FeatureChainExpression.operand += current}` inside a repetition
    // (KerMLExpressions.xtext:301, :319), which is the Xtext idiom for a left fold. A
    // left operand restricted to non-chains would forbid `a.b.c` outright.
    //
    // The loop is why this is iterative rather than recursive: a chain of ten thousand
    // links is one stack frame, and invariant 3 is that no input kills the parser.
    //
    // NO EmptyResultMember, although the metaclass IS an OperatorExpression (8.3.4.8.4)
    // and every operator in the infix table owns one. The BNF writes EmptyResultMember
    // explicitly where a production has one — BinaryOperatorExpression and
    // FeatureReferenceExpression both name it — and this production does not. Adding one
    // by analogy would put an element in the tree that the grammar does not state.
    //
    // FeatureChainMember = FeatureReferenceMember | OwnedFeatureChainMember, and is NOT
    // marked: the second alternative, a FeatureChain of two or more links, is absent.
    // With the left fold it is also unreachable here — every member this loop reads is a
    // single link, because the accumulated chain is the LEFT operand. It is reachable
    // from SysML's AssignmentActionUsage (8.2.2.17.5), which is unimplemented.
    // production: BracketExpression
    //
    // BracketExpression =
    //     ownedRelationship += PrimaryArgumentMember operator = '['
    //     ownedRelationship += SequenceExpressionListMember ']'   (KerML 8.2.5.8.2)
    //
    // production: PrimaryArgumentMember
    //
    // PrimaryArgumentMember = ownedMemberParameter = PrimaryArgument
    //                                                            (KerML 8.2.5.8.2)
    //
    // The quantity form: `1200 [kg]`. Postfix and left-folding like the chain, and in
    // the same loop because the Pilot puts them in the same loop
    // (KerMLExpressions.xtext:299–322) and because `a.b[kg]` and `a[1].b` both have to
    // work.
    //
    // No EmptyResultMember, for the reason FeatureChainExpression has none: the BNF
    // names one where a production has one, and 8.2.5.8.2 does not.
    //
    // deviations.json buckets this spec_only/follow_spec, whose generic rationale says
    // to expect the corpus not to exercise it. That is wrong for THIS production and the
    // record now says so: the corpus writes a bracketed quantity in hundreds of places,
    // and the Pilot implements the form — it simply does not give the rule a name,
    // inlining it as `{OperatorExpression.operand += current} operator = '['`
    // (KerMLExpressions.xtext:307). The bucket is a naming difference, not a gap.
    fn bracket_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::BracketExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.sequence_expression_list_member();
        self.expect(SyntaxKind::RBracket, "`]`");
        self.finish_node();
    }

    /// One `FeatureChainExpression`, folded over what is already at `start`.
    fn feature_chain_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::FeatureChainExpression);
        self.wrap_at(start, &NON_FEATURE_CHAIN_PRIMARY_ARGUMENT);
        self.bump();
        self.kerml_feature_chain_member();
        self.finish_node();
    }

    /// The postfix layer of `PrimaryExpression`, folded left over `start`.
    ///
    /// `FeatureChainExpression` and `BracketExpression` are both written after their
    /// operand and both take that operand as a `PrimaryArgument`, so they nest in
    /// whatever order they are written: `a.b[kg]` is a bracket over a chain and `a[1].b`
    /// is a chain over a bracket. One loop is what makes that fall out rather than being
    /// arranged.
    ///
    /// The remaining postfix forms of 8.2.5.8.2 — `IndexExpression` (`#(`),
    /// `FunctionOperationExpression` (`->`), `CollectExpression` and `SelectExpression` —
    /// are absent, each with a rejection case.
    fn postfix_tail(&mut self, start: rowan::Checkpoint) {
        let mut levels: u32 = 0;
        while self.at_feature_chain() || self.at(SyntaxKind::LBracket) {
            // BOUNDED, although this loop uses no stack of its own. Every level wraps
            // what is already there, so the TREE is as deep as the expression is long
            // even when the parser's own recursion is flat — and a consumer walking a
            // 50000-level tree overflows on a thread with a 2 MiB stack, which aborts
            // rather than panics (invariant 3). Folding also stops being linear at that
            // size. The depth counter is the mechanism the rest of the parser already
            // uses, so a chain is counted against the same budget as a nested body.
            if self.depth >= MAX_DEPTH {
                self.report_too_deep();
                break;
            }
            self.depth += 1;
            levels += 1;
            if self.at(SyntaxKind::LBracket) {
                self.bracket_expression(start);
            } else {
                self.feature_chain_expression(start);
            }
        }
        // The levels belong to this expression, not to anything enclosing it, so the
        // budget is returned when the run ends. A `.` or `[` past the limit is left
        // where it stands and reaches the tree through the enclosing body's recovery,
        // which is what keeps the text lossless.
        self.depth -= levels;
    }

    /// Whether a `'.'` here opens a `FeatureChainExpression` rather than something else.
    ///
    /// Three other productions put a `.` after a primary expression, and none of them is
    /// a chain:
    ///
    /// ```text
    /// SelectExpression          = PrimaryArgumentMember '.?' BodyArgumentMember
    /// CollectExpression         = PrimaryArgumentMember '.'  BodyArgumentMember
    /// MetadataAccessExpression  = ElementReferenceMember '.' 'metadata'
    /// ```
    ///
    /// `.?` lexes as one token, so a select is already not a `Dot`. The other two are
    /// separated by what FOLLOWS the dot: a collect takes a `BodyExpression`, which opens
    /// on `'{'`, and a metadata access takes the keyword `metadata`. A
    /// `FeatureChainMember` reaches a `QualifiedName`, which opens on a NAME — and a
    /// keyword is not a name (`KerML` 8.2.2.6), so asking for a name excludes both.
    ///
    /// All three are unimplemented. This check is what keeps them that way rather than
    /// letting a chain quietly accept text it is not: `E.metadata` reads as a metadata
    /// access or not at all, never as a feature called `metadata`.
    fn at_feature_chain(&self) -> bool {
        self.at(SyntaxKind::Dot) && self.nth_is_name(1)
    }

    /// `KerML`'s `FeatureChainMember` after the dot, in its `FeatureReferenceMember` form.
    ///
    /// SCOPED IN THE NAME because `SysML` states a production of the same name with a
    /// different body — `memberElement = [QualifiedName] | OwnedFeatureChainMember`
    /// (8.2.2.17.5), read by `sysml_feature_chain_member` — where `KerML`'s is
    /// `FeatureReferenceMember | OwnedFeatureChainMember` (8.2.5.8.2). Two units, not one
    /// (ADR-0015), and one Rust name for both would hide that.
    ///
    /// The nodes are `FeatureReferenceMember` and `FeatureReference`, both of which
    /// `feature_reference_expression` also builds and marks. They are built here without
    /// the `FeatureReferenceExpression` around them and without the `EmptyResultMember`
    /// after them, because neither is in this production.
    fn kerml_feature_chain_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureReferenceMember);
        self.start_node(SyntaxKind::FeatureReference);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
    }

    /// `PrimaryExpression`'s alternatives other than `FeatureChainExpression`.
    fn non_feature_chain_primary_expression(&mut self) {
        // NullExpression = 'null' | '(' ')' — the empty pair is decided before
        // SequenceExpression, which would otherwise read the '(' and find no
        // expression.
        if self.at_keyword("null") || self.at_empty_parentheses() {
            self.null_expression();
        } else if self.at(SyntaxKind::LParen) {
            self.sequence_expression();
        } else if self.at_literal_expression() {
            self.literal_expression();
        } else if self.at_invocation_expression() {
            // BEFORE the feature reference, and the two are told apart by ONE token:
            // both open on a QualifiedName and only an invocation has a `(` after it.
            self.invocation_expression();
        } else if self.at_feature_reference() {
            self.feature_reference_expression();
        } else {
            self.error_expected("an expression");
        }
    }

    /// Whether a `'('` here is immediately closed, making it a `NullExpression`.
    fn at_empty_parentheses(&self) -> bool {
        self.at(SyntaxKind::LParen) && self.nth_is(1, SyntaxKind::RParen)
    }

    // production: NullExpression
    //
    // NullExpression : NullExpression = 'null' | '(' ')'         (KerML 8.2.5.8.3)
    fn null_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NullExpression);
        if self.at_keyword("null") {
            self.bump_as(SyntaxKind::KwNull);
        } else {
            self.expect(SyntaxKind::LParen, "`(`");
            self.expect(SyntaxKind::RParen, "`)`");
        }
        self.finish_node();
    }

    // production: SequenceExpression
    //
    // SequenceExpression = '(' SequenceExpressionList ')'        (KerML 8.2.5.8.2)
    fn sequence_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpression);
        self.expect(SyntaxKind::LParen, "`(`");
        self.sequence_expression_list();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    // production: SequenceExpressionList
    //
    // SequenceExpressionList = OwnedExpression ','?
    //                        | SequenceOperatorExpression        (KerML 8.2.5.8.2)
    //
    // production: SequenceOperatorExpression
    //
    // SequenceOperatorExpression : OperatorExpression =
    //     ownedRelationship += OwnedExpressionMember
    //     operator = ','
    //     ownedRelationship += SequenceExpressionListMember      (KerML 8.2.5.8.2)
    //
    // Both alternatives are an expression followed by a `','`, so which one it is
    // depends on what comes after the comma: another expression makes it a
    // SequenceOperatorExpression, and a `')'` makes it the first alternative's
    // trailing comma. The node is opened retroactively once that is known.
    fn sequence_expression_list(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpressionList);
        let start = self.builder.checkpoint();
        self.owned_expression();
        if self.at(SyntaxKind::Comma) {
            if self.nth_is(1, SyntaxKind::RParen) {
                // `( a , )` — the trailing `','?` of the first alternative.
                self.bump();
            } else {
                self.start_node_at(start, SyntaxKind::SequenceOperatorExpression);
                self.wrap_at(start, &[SyntaxKind::OwnedExpressionMember]);
                self.bump();
                self.sequence_expression_list_member();
                self.finish_node();
            }
        }
        self.finish_node();
    }

    // production: SequenceExpressionListMember
    //
    // SequenceExpressionListMember : OwningMembership =
    //     ownedRelatedElement += SequenceExpressionList          (KerML 8.2.5.8.2)
    fn sequence_expression_list_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpressionListMember);
        self.sequence_expression_list();
        self.finish_node();
    }

    /// Whether a `FeatureReferenceExpression` starts here.
    ///
    /// `FeatureReference = [QualifiedName]`, and a `QualifiedName` opens with a NAME
    /// or with the `'$'` of its global-scope prefix (`KerML` 8.2.3.4.1).
    fn at_feature_reference(&self) -> bool {
        self.at_name() || self.at(SyntaxKind::Dollar)
    }

    /// The index just past a `QualifiedName` written at the `n`th meaningful token, or
    /// `None` if one is not written there.
    ///
    /// `QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME` (`KerML` 8.2.3.4.1), walked
    /// exactly as `qualified_name` consumes it — including the two-token test that a
    /// `::` belongs to the name only when a NAME follows it, so `A::*` ends at `A`.
    /// A recogniser that walked it differently from the parser would accept a prefix the
    /// parser then failed to read.
    fn skip_qualified_name(&self, n: usize) -> Option<usize> {
        let mut n = n;
        if self
            .peek_nth(n)
            .is_some_and(|t| t.kind == SyntaxKind::Dollar)
        {
            n += 1;
            if !self.nth_is(n, SyntaxKind::ColonColon) {
                return None;
            }
            n += 1;
        }
        if !self.nth_is_name(n) {
            return None;
        }
        n += 1;
        while self.nth_is(n, SyntaxKind::ColonColon) && self.nth_is_name(n + 1) {
            n += 2;
        }
        Some(n)
    }

    /// Whether an `InvocationExpression` starts here.
    ///
    /// A `QualifiedName` with a `'('` after it — the whole of what separates it from a
    /// `FeatureReferenceExpression`, which is the same name with nothing after it
    /// (`KerML` 8.2.5.8.3).
    ///
    /// It asks for a NAME rather than for any token before the `(`, which is what keeps
    /// `x and (y)` an operator over a parenthesised operand: `and` is reserved
    /// (`SysML` 8.2.2.1.2) and a keyword is not a name, so `nth_is_name` says no. A `(`
    /// with nothing before it never reaches here at all — `null_expression` and
    /// `sequence_expression` are asked first.
    fn at_invocation_expression(&self) -> bool {
        self.skip_qualified_name(0)
            .is_some_and(|n| self.nth_is(n, SyntaxKind::LParen))
    }

    // production: InvocationExpression
    //
    // InvocationExpression : InvocationExpression =
    //     ownedRelationship += InstantiatedTypeMember
    //     ArgumentList
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.3)
    //
    // production: InstantiatedTypeReference
    //
    // InstantiatedTypeReference : Type = [QualifiedName]         (KerML 8.2.5.8.3)
    //
    // InstantiatedTypeMember is NOT marked. It is
    //
    //     InstantiatedTypeMember = memberElement = InstantiatedTypeReference
    //                            | OwnedFeatureChainMember       (KerML 8.2.5.8.3)
    //
    // and only the first alternative is read here. The second is a FeatureChain — `a.b(x)`
    // — and it is absent for the same reason FeatureChainMember's chain alternative is:
    // the chain productions read a single link, and a multi-link chain in this position
    // has no caller yet. The node is still built, because the membership is in the tree
    // either way; what is not claimed is the alternation.
    //
    // The EmptyResultMember is the result parameter every invocation owns and nobody
    // writes, exactly as FeatureReferenceExpression owns one (8.2.5.8.3 names it in both).
    fn invocation_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InvocationExpression);
        self.start_node(SyntaxKind::InstantiatedTypeMember);
        self.start_node(SyntaxKind::InstantiatedTypeReference);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.argument_list();
        self.empty_result_member();
        self.finish_node();
    }

    // production: ArgumentList
    //
    // ArgumentList = '(' ( PositionalArgumentList | NamedArgumentList )? ')'
    //                                                            (KerML 8.2.5.8.3)
    //
    // The two lists are ALTERNATIVES, so a list commits to one of them and a mixed list
    // is not this grammar — tests/rejection/argument-list-does-not-mix-positional-and-
    // named.sysml and its mirror hold both directions.
    //
    // `at_named_argument` decides which, and it is one token of lookahead past a
    // QualifiedName: a NamedArgument is `ParameterRedefinition '=' ...` and a positional
    // argument is an OwnedExpression, which cannot contain a bare `=` because `=` is not
    // in the precedence table of 8.2.5.8.1 (docs/operator-precedence.toml, which a test
    // diffs against the parser's INFIX table both ways).
    fn argument_list(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ArgumentList);
        self.expect(SyntaxKind::LParen, "`(`");
        if !self.at(SyntaxKind::RParen) {
            if self.at_named_argument() {
                self.named_argument_list();
            } else {
                self.positional_argument_list();
            }
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    /// Whether a `NamedArgument` rather than a positional one is written here.
    ///
    /// `NamedArgument = ParameterRedefinition '=' ArgumentValue` (`KerML` 8.2.5.8.3),
    /// and `ParameterRedefinition` is a `QualifiedName`, so the question is whether a
    /// single `'='` follows one. The `=` is checked by its own token kind, so the `==`
    /// of an equality expression is a different token and does not answer this.
    fn at_named_argument(&self) -> bool {
        self.skip_qualified_name(0)
            .is_some_and(|n| self.nth_is(n, SyntaxKind::Eq))
    }

    // production: PositionalArgumentList
    //
    // PositionalArgumentList =
    //     ownedRelationship += ArgumentMember
    //     ( ',' ownedRelationship += ArgumentMember )*           (KerML 8.2.5.8.3)
    //
    // NO trailing comma, unlike the SequenceExpressionList three clauses away, which
    // states `','?` and does admit `( a , )` —
    // tests/rejection/argument-list-takes-no-trailing-comma.sysml is that difference.
    //
    // TIER_LOOSEST because an ArgumentMember's value is a whole OwnedExpression
    // (ArgumentValue, 8.2.5.8.1): the `(` and `)` bound it, so no operator inside can
    // reach past them and nothing needs to be held back from the climb.
    fn positional_argument_list(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PositionalArgumentList);
        self.argument_member(TIER_LOOSEST);
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.argument_member(TIER_LOOSEST);
        }
        self.finish_node();
    }

    // production: NamedArgumentList
    //
    // NamedArgumentList =
    //     ownedRelationship += NamedArgumentMember
    //     ( ',' ownedRelationship += NamedArgumentMember )*      (KerML 8.2.5.8.3)
    fn named_argument_list(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NamedArgumentList);
        self.named_argument_member();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.named_argument_member();
        }
        self.finish_node();
    }

    // production: NamedArgumentMember
    //
    // NamedArgumentMember : FeatureMembership =
    //     ownedMemberFeature = NamedArgument                     (KerML 8.2.5.8.3)
    //
    // A FeatureMembership, and NOT the ParameterMembership its positional sibling
    // ArgumentMember is (8.2.5.8.1). The two argument forms therefore differ in the
    // abstract syntax and not only in the text. What the grammar states about the
    // difference and this comment does not infer past: a NamedArgument names the
    // parameter it supplies through a ParameterRedefinition, and an ArgumentMember has
    // no such relationship.
    //
    // production: NamedArgument
    //
    // NamedArgument : Feature =
    //     ownedRelationship += ParameterRedefinition '='
    //     ownedRelationship += ArgumentValue                     (KerML 8.2.5.8.3)
    //
    // production: ParameterRedefinition
    //
    // ParameterRedefinition : Redefinition =
    //     redefinedFeature = [QualifiedName]                     (KerML 8.2.5.8.3)
    //
    // The ArgumentValue node is built here rather than by `argument_member`, because a
    // NamedArgument owns its value directly and has no Argument between the two.
    fn named_argument_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NamedArgumentMember);
        self.start_node(SyntaxKind::NamedArgument);
        self.start_node(SyntaxKind::ParameterRedefinition);
        self.qualified_name();
        self.finish_node();
        self.expect(SyntaxKind::Eq, "`=` after a named argument's parameter");
        self.start_node(SyntaxKind::ArgumentValue);
        self.owned_expression();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: FeatureReferenceExpression
    //
    // FeatureReferenceExpression : FeatureReferenceExpression =
    //     FeatureReferenceMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.3)
    //
    // production: FeatureReferenceMember
    //
    // FeatureReferenceMember : Membership =
    //     memberElement = FeatureReference                       (KerML 8.2.5.8.3)
    //
    // production: FeatureReference
    //
    // FeatureReference : Feature = [QualifiedName]               (KerML 8.2.5.8.3)
    fn feature_reference_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureReferenceExpression);
        self.start_node(SyntaxKind::FeatureReferenceMember);
        self.start_node(SyntaxKind::FeatureReference);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.empty_result_member();
        self.finish_node();
    }

    /// Whether a `LiteralExpression` starts here (`KerML` 8.2.5.8.4).
    ///
    /// The five literals, by their opening token: `'true'`/`'false'`, a
    /// `STRING_VALUE`, a `DECIMAL_VALUE`, the `'.'` or `EXPONENTIAL_VALUE` a
    /// `RealValue` may open with, and the `'*'` of `LiteralInfinity`.
    fn at_literal_expression(&self) -> bool {
        self.at(SyntaxKind::StringValue)
            || self.at(SyntaxKind::DecimalValue)
            || self.at(SyntaxKind::ExponentialValue)
            || self.at(SyntaxKind::Dot)
            || self.at(SyntaxKind::Star)
            || self.at_keyword("true")
            || self.at_keyword("false")
    }

    // LiteralExpression = LiteralBoolean | LiteralString | LiteralInteger
    //                   | LiteralReal | LiteralInfinity          (KerML 8.2.5.8.4)
    //
    // NOT marked for coverage: it is an alternation with no method that is it, as
    // FeatureSpecialization is. All five alternatives below are marked.
    fn literal_expression(&mut self) {
        if self.at_keyword("true") || self.at_keyword("false") {
            self.literal_boolean();
        } else if self.at(SyntaxKind::StringValue) {
            self.literal_string();
        } else if self.at(SyntaxKind::Star) {
            self.literal_infinity();
        } else if self.at_literal_real() {
            self.literal_real();
        } else {
            self.literal_integer();
        }
    }

    /// Whether the literal here is a `RealValue` rather than a `DECIMAL_VALUE`.
    ///
    /// `RealValue = DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE )
    /// | EXPONENTIAL_VALUE` (`KerML` 8.2.5.8.4). The lexer does not take a `'.'`
    /// into a number, so `1.5` arrives as three tokens and the `'.'` is what
    /// separates a real from an integer. `1..5` is not one of them: `'..'` is a
    /// single token by maximal munch, so the range operator never looks like a
    /// decimal point.
    fn at_literal_real(&self) -> bool {
        if self.at(SyntaxKind::ExponentialValue) || self.at(SyntaxKind::Dot) {
            return true;
        }
        self.at(SyntaxKind::DecimalValue)
            && self.nth_is(1, SyntaxKind::Dot)
            && (self.nth_is(2, SyntaxKind::DecimalValue)
                || self.nth_is(2, SyntaxKind::ExponentialValue))
    }

    // production: LiteralBoolean
    //
    // LiteralBoolean : LiteralBoolean = value = BooleanValue     (KerML 8.2.5.8.4)
    // BooleanValue = 'true' | 'false'                            (KerML 8.2.5.8.4)
    fn literal_boolean(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LiteralBoolean);
        if self.at_keyword("true") {
            self.bump_as(SyntaxKind::KwTrue);
        } else {
            self.bump_as(SyntaxKind::KwFalse);
        }
        self.finish_node();
    }

    // production: LiteralString
    //
    // LiteralString : LiteralString = value = STRING_VALUE       (KerML 8.2.5.8.4)
    fn literal_string(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LiteralString);
        self.expect(SyntaxKind::StringValue, "a string");
        self.finish_node();
    }

    // production: LiteralInteger
    //
    // LiteralInteger : LiteralInteger = value = DECIMAL_VALUE    (KerML 8.2.5.8.4)
    fn literal_integer(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LiteralInteger);
        self.expect(SyntaxKind::DecimalValue, "an integer");
        self.finish_node();
    }

    // production: LiteralReal
    //
    // LiteralReal : LiteralReal = value = RealValue              (KerML 8.2.5.8.4)
    //
    // RealValue = DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE )
    //           | EXPONENTIAL_VALUE                              (KerML 8.2.5.8.4)
    //
    // RealValue is a production and not a terminal, so a real is up to three tokens
    // and the node is what holds them together. The leading DECIMAL_VALUE is
    // optional, which makes `.5` a real; the trailing part is not, which makes `1.`
    // a reported error rather than a real.
    fn literal_real(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LiteralReal);
        if self.at(SyntaxKind::ExponentialValue) {
            self.bump();
            self.finish_node();
            return;
        }
        if self.at(SyntaxKind::DecimalValue) {
            self.bump();
        }
        self.expect(SyntaxKind::Dot, "`.`");
        if self.at(SyntaxKind::DecimalValue) || self.at(SyntaxKind::ExponentialValue) {
            self.bump();
        } else {
            self.error_expected("a digit after `.`");
        }
        self.finish_node();
    }

    // production: LiteralInfinity
    //
    // LiteralInfinity : LiteralInfinity = '*'                    (KerML 8.2.5.8.4)
    //
    // The same `'*'` that is multiplication at tier 4. Position separates them and
    // no lookahead is needed: an operand position reaches here, and an operator
    // position reaches `infix_operator_here`, so `[0..*]` and `a * b` both read the
    // one token correctly.
    fn literal_infinity(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LiteralInfinity);
        self.expect(SyntaxKind::Star, "`*`");
        self.finish_node();
    }

    // production: ArgumentMember
    //
    // ArgumentMember : ParameterMembership =
    //     ownedMemberParameter = Argument                        (KerML 8.2.5.8.1)
    //
    // production: Argument
    //
    // Argument : Feature = ownedRelationship += ArgumentValue    (KerML 8.2.5.8.1)
    //
    // production: ArgumentValue
    //
    // ArgumentValue : FeatureValue = value = OwnedExpression     (KerML 8.2.5.8.1)
    fn argument_member(&mut self, tier: u8) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ArgumentMember);
        self.start_node(SyntaxKind::Argument);
        self.start_node(SyntaxKind::ArgumentValue);
        self.expression(tier);
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: ArgumentExpressionMember
    //
    // ArgumentExpressionMember : ParameterMembership =
    //     ownedRelatedElement += ArgumentExpression              (KerML 8.2.5.8.1)
    //
    // production: ArgumentExpression
    //
    // ArgumentExpression : Feature =
    //     ownedRelationship += ArgumentExpressionValue           (KerML 8.2.5.8.1)
    //
    // production: ArgumentExpressionValue
    //
    // ArgumentExpressionValue : FeatureValue =
    //     value = OwnedExpressionReference                       (KerML 8.2.5.8.1)
    //
    // production: OwnedExpressionReference
    //
    // OwnedExpressionReference : FeatureReferenceExpression =
    //     ownedRelationship += OwnedExpressionMember             (KerML 8.2.5.8.1)
    //
    // production: OwnedExpressionMember
    //
    // OwnedExpressionMember : FeatureMembership =
    //     ownedFeatureMember = OwnedExpression                   (KerML 8.2.5.8.1)
    //
    // The operand of a ConditionalBinaryOperator and of a conditional's branches.
    // Five memberships rather than the three an ArgumentMember has, and the extra two
    // are the point: the expression is REFERENCED here, not evaluated as an argument,
    // which is how the abstract syntax records that `??`, `or`, `and` and `implies`
    // short-circuit and `|`, `&` and `xor` do not.
    fn argument_expression_member(&mut self, tier: u8) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ArgumentExpressionMember);
        self.start_node(SyntaxKind::ArgumentExpression);
        self.start_node(SyntaxKind::ArgumentExpressionValue);
        self.start_node(SyntaxKind::OwnedExpressionReference);
        self.start_node(SyntaxKind::OwnedExpressionMember);
        self.expression(tier);
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: TypeReference
    //
    // TypeReference : Feature =
    //     ownedRelationship += ReferenceTyping                   (KerML 8.2.5.8.1)
    //
    // production: ReferenceTyping
    //
    // ReferenceTyping : FeatureTyping = type = [QualifiedName]   (KerML 8.2.5.8.1)
    //
    // production: TypeReferenceMember
    //
    // TypeReferenceMember : FeatureMembership =
    //     ownedMemberFeature = TypeReference                     (KerML 8.2.5.8.1)
    //
    // production: TypeResultMember
    //
    // TypeResultMember : ReturnParameterMembership =
    //     ownedMemberFeature = TypeReference                     (KerML 8.2.5.8.1)
    //
    // The two memberships differ in the abstract syntax and not in the text: a test
    // names its type through a FeatureMembership and a cast through a
    // ReturnParameterMembership, because a cast's type IS its result. `member` is
    // which one the operator named.
    fn type_reference_member(&mut self, member: SyntaxKind) {
        self.eat_trivia();
        self.start_node(member);
        self.start_node(SyntaxKind::TypeReference);
        self.start_node(SyntaxKind::ReferenceTyping);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: EmptyResultMember
    //
    // EmptyResultMember : ReturnParameterMembership =
    //     ownedRelatedElement += EmptyFeature                    (KerML 8.2.5.8.1)
    //
    // production: EmptyFeature
    //
    // EmptyFeature : Feature = { }                               (KerML 8.2.5.8.1)
    //
    // No tokens, as EmptyMultiplicity has none (SysML 8.2.2.9.1): every
    // OperatorExpression owns a result parameter that is written nowhere in the text.
    // Deliberately without `eat_trivia`, because a node that consumes nothing must
    // not pull the trivia after the operand inside itself.
    fn empty_result_member(&mut self) {
        self.start_node(SyntaxKind::EmptyResultMember);
        self.start_node(SyntaxKind::EmptyFeature);
        self.finish_node();
        self.finish_node();
    }

    // -- reading the precedence table --------------------------------------------

    /// The infix operator written here, or `None` if the next token is not one.
    ///
    /// Symbols are checked before words, as `at_feature_specialization` does: the
    /// lexer gives each symbol its own kind while every word arrives as a
    /// `BasicName`, so a symbol is one comparison and a word is a text match.
    fn infix_operator_here(&self) -> Option<&'static InfixOperator> {
        INFIX.iter().find(|op| match op.spelling {
            Spelling::Symbol(kind) => self.at(kind),
            Spelling::Word(word) => self.at_keyword(word),
        })
    }

    /// Whether a `UnaryOperator` is written here (`KerML` 8.2.5.8.1).
    fn at_unary_operator(&self) -> bool {
        UNARY_OPERATORS.iter().any(|spelling| match spelling {
            Spelling::Symbol(kind) => self.at(*kind),
            Spelling::Word(word) => self.at_keyword(word),
        })
    }

    /// Whether a `ClassificationExpression` opens here with no left operand.
    ///
    /// Its `ArgumentMember` is the one optional operand in the clause, so a
    /// classification or cast operator may be the first token of an expression.
    fn at_leading_classification(&self) -> bool {
        self.at(SyntaxKind::At)
            || ["istype", "hastype", "as"]
                .iter()
                .any(|word| self.at_keyword(word))
    }

    /// Whether a `MetaclassificationExpression` starts here.
    ///
    /// Decided by looking past the `QualifiedName` that is its left operand: `x meta
    /// T` is one and `x` alone is a `FeatureReferenceExpression`, and the two differ
    /// only after the name. `at_metaclassification` is asked before the name is
    /// parsed, because a `MetadataArgumentMember` cannot be wrapped around a
    /// `FeatureReferenceExpression` that is already in the tree.
    fn at_metaclassification(&self) -> bool {
        if !self.at_feature_reference() {
            return false;
        }
        let n = self.after_qualified_name(0);
        self.nth_is(n, SyntaxKind::AtAt) || self.nth_is_keyword(n, "meta")
    }

    /// The index just past the `QualifiedName` written from the `n`th token.
    ///
    /// `QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME` (`KerML` 8.2.3.4.1). A
    /// `'::'` is only part of the name when a NAME follows it, which is the rule
    /// `qualified_name` itself applies.
    fn after_qualified_name(&self, n: usize) -> usize {
        let mut n = n;
        if self.nth_is(n, SyntaxKind::Dollar) && self.nth_is(n + 1, SyntaxKind::ColonColon) {
            n += 2;
        }
        if !self.peek_nth(n).is_some_and(|token| self.is_name(token)) {
            return n;
        }
        n += 1;
        while self.nth_is(n, SyntaxKind::ColonColon)
            && self
                .peek_nth(n + 1)
                .is_some_and(|token| self.is_name(token))
        {
            n += 2;
        }
        n
    }

    /// Consume the operator token of a `ClassificationExpression`.
    fn bump_classification_operator(&mut self) {
        if self.at(SyntaxKind::At) {
            self.bump();
        } else if let Some(word) = ["istype", "hastype", "as"]
            .into_iter()
            .find(|word| self.at_keyword(word))
        {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        } else {
            self.error_expected("`istype`, `hastype`, `@` or `as`");
        }
    }

    /// Consume the operator token of a `UnaryOperatorExpression`.
    fn bump_unary_operator(&mut self) {
        match UNARY_OPERATORS.iter().find(|spelling| match spelling {
            Spelling::Symbol(kind) => self.at(*kind),
            Spelling::Word(word) => self.at_keyword(word),
        }) {
            Some(spelling) => self.bump_spelling(*spelling),
            None => self.error_expected("`+`, `-`, `~` or `not`"),
        }
    }

    /// Consume the token an operator is spelled with, tagged as the token set names it.
    fn bump_spelling(&mut self, spelling: Spelling) {
        match spelling {
            Spelling::Symbol(_) => self.bump(),
            Spelling::Word(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
        }
    }

    // -- retroactive nodes -------------------------------------------------------

    /// Open `kind` retroactively at `start`, over what is already in the tree.
    fn start_node_at(&mut self, start: rowan::Checkpoint, kind: SyntaxKind) {
        self.builder
            .start_node_at(start, Sv2Language::kind_to_raw(kind));
    }

    /// Wrap what is already in the tree at `start` in `kinds`, outermost first.
    ///
    /// rowan's parent stack finishes in reverse, so the first kind opened is the
    /// outermost one and `&[ArgumentMember, Argument, ArgumentValue]` nests in the
    /// order the clause writes them.
    fn wrap_at(&mut self, start: rowan::Checkpoint, kinds: &[SyntaxKind]) {
        for kind in kinds {
            self.start_node_at(start, *kind);
        }
        for _ in kinds {
            self.finish_node();
        }
    }

    /// Whether the `n`th meaningful token from here is of `kind`.
    fn nth_is(&self, n: usize, kind: SyntaxKind) -> bool {
        self.peek_nth(n).is_some_and(|token| token.kind == kind)
    }

    // production: UsageBody
    //
    // UsageBody : Usage = DefinitionBody                         (SysML 8.2.2.6.2)
    fn usage_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageBody);
        self.definition_body();
        self.finish_node();
    }

    // production: PortDefinition
    //
    // PortDefinition = DefinitionPrefix 'port' 'def' Definition
    //                  ownedRelationship += ConjugatedPortDefinitionMember
    //                                                            (SysML 8.2.2.12)
    //
    // Not in SIMPLE_DEFINITIONS, and the one part that keeps it out consumes no tokens.
    // Every port definition implicitly declares its conjugate — `port def P;` gives you
    // `~P` — so reading it with the shared spine would accept the text and silently drop
    // three elements the abstract syntax says are there.
    //
    // A DefinitionPrefix, not an OccurrenceDefinitionPrefix: a port is not an occurrence,
    // so `individual port def P;` is an error, as it is for an attribute.
    fn port_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PortDefinition);
        self.definition_prefix();
        self.expect_keyword("port");
        self.expect_keyword("def");
        self.definition();
        self.conjugated_port_definition_member();
        self.finish_node();
    }

    // production: ConjugatedPortDefinitionMember
    //
    // ConjugatedPortDefinitionMember : OwningMembership =
    //     ownedRelatedElement += ConjugatedPortDefinition        (SysML 8.2.2.12)
    //
    // production: ConjugatedPortDefinition
    //
    // ConjugatedPortDefinition = ownedRelationship += PortConjugation
    //
    // production: PortConjugation
    //
    // PortConjugation = { }
    //
    // Three nested nodes and not one token, exactly as EmptyMultiplicityMember is two.
    // The nodes are built rather than omitted so the tree carries the elements the
    // abstract syntax puts there; a consumer asking a port definition for its conjugate
    // finds it, rather than having to know to synthesise one.
    fn conjugated_port_definition_member(&mut self) {
        self.start_node(SyntaxKind::ConjugatedPortDefinitionMember);
        self.start_node(SyntaxKind::ConjugatedPortDefinition);
        self.start_node(SyntaxKind::PortConjugation);
        self.finish_node();
        self.finish_node();
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

    // production: RequirementBody
    //
    // RequirementBody : Type = ';' | '{' RequirementBodyItem* '}'
    //                                                            (SysML 8.2.2.21.1)
    //
    // RequirementBodyItem is NOT marked. It is
    //
    //     DefinitionBodyItem | SubjectMember | RequirementConstraintMember
    //     | FramedConcernMember | RequirementVerificationMember | ActorMember
    //     | StakeholderMember
    //
    // — a SUPERSET of DefinitionBodyItem, and that is the whole reason this body is
    // reachable at the cost of one method. The six extra members are unimplemented, so
    // `subject`, `require`, `assume`, `frame`, `verify`, `actor` and `stakeholder` at
    // member position are reported by the body's recovery like any other text the
    // parser does not yet read. Held as files by
    // tests/rejection/requirement-body-subject-member-is-not-implemented.sysml and
    // tests/rejection/requirement-body-constraint-member-is-not-implemented.sysml.
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

    // production: ActionDefinition
    //
    // ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
    //                    DefinitionDeclaration ActionBody        (SysML 8.2.2.17.1)
    //
    // Off the SIMPLE_DEFINITIONS spine for the reason RequirementDefinition and
    // ConstraintDefinition are: it names the declaration and the body separately rather
    // than taking a Definition, so there is no Definition node in its tree.
    //
    // The metaclass is SysML::ActionDefinition (8.3.17.3), both a Behavior and an
    // OccurrenceDefinition. checkActionDefinitionSpecialization is an implied
    // specialization and belongs in sv2-hir, not here (ADR-0002).
    fn action_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("action");
        self.expect_keyword("def");
        self.definition_declaration();
        self.action_body();
        self.finish_node();
    }

    // production: ActionBody
    //
    // ActionBody : Type = ';' | '{' ActionBodyItem* '}'          (SysML 8.2.2.17.1)
    //
    // ActionBodyItem is NOT marked, and it is the largest unimplemented thing left in
    // this grammar:
    //
    //     ActionBodyItem = NonBehaviorBodyItem
    //                    | InitialNodeMember ActionTargetSuccessionMember*
    //                    | SourceSuccessionMember? ActionBehaviorMember
    //                      ActionTargetSuccessionMember*
    //                    | GuardedSuccessionMember
    //
    // The first alternative is read in the part this parser already had:
    // NonBehaviorBodyItem is Import | AliasMember | DefinitionMember | VariantUsageMember
    // | NonOccurrenceUsageMember | SourceSuccessionMember? StructureUsageMember
    // (8.2.2.17.1), and the first three are the three a definition body reads. The
    // second is read whole in its TargetSuccession form. The third is read over the
    // behaviour usages that exist (ActionUsage, PerformActionUsage) and over the one
    // ActionNode that does, ControlNode — `merge`, `decide`, `join`, `fork`. What is
    // still reported where it stands: the other ActionNodes (`accept`, `send`,
    // `assign`, `terminate`, `if`, `while`, `for`), GuardedSuccessionMember, and the
    // guarded and default target successions.
    fn action_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Action);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after an action definition declaration");
        }
        self.finish_node();
    }

    // production: ActionUsage
    //
    // ActionUsage = OccurrenceUsagePrefix 'action'
    //               ActionUsageDeclaration ActionBody            (SysML 8.2.2.17.2)
    //
    // production: ActionUsageDeclaration
    //
    // ActionUsageDeclaration : ActionUsage =
    //     UsageDeclaration ValuePart?                            (SysML 8.2.2.17.2)
    //
    // Off the SIMPLE_USAGES spine, and the difference is at the END rather than the
    // prefix: the seven take `Usage`, which is `UsageDeclaration UsageCompletion`, and
    // this takes a declaration of its own and then an ActionBody. The tokens are the
    // same ones in the same order; what differs is which body rule reads the braces, and
    // reading an action body against DefinitionBody would accept the declaration and
    // then read its contents against the wrong item set.
    //
    // ActionUsageDeclaration has a body identical to ConstraintUsageDeclaration
    // (8.2.2.20). They are two productions rather than one because they belong to two
    // metaclasses, and each is built here under its own name so the tree says which was
    // taken.
    //
    // The metaclass is SysML::ActionUsage (8.3.17.4), both a Step and an
    // OccurrenceUsage.
    fn action_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("action");
        self.action_usage_declaration();
        self.action_body();
        self.finish_node();
    }

    fn action_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionUsageDeclaration);
        self.usage_declaration();
        if self.at_value_part() {
            self.value_part();
        }
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

    /// Whether a `FlowUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'flow'` with no `def` after it (`SysML` 8.2.2.16): the
    /// `def` is what makes it the `FlowDefinition` beside it, as for every usage.
    fn at_flow_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "flow") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: FlowUsage@sysml
    //
    // FlowUsage = OccurrenceUsagePrefix 'flow' FlowDeclaration DefinitionBody
    //                                                            (SysML 8.2.2.16)
    //
    // A StructureUsageElement (8.2.2.6.4), so it is owned as `part` is: through a
    // StructureUsageMember in an action body, an OccurrenceUsageMember in a definition
    // body, a PackageMember in a package. The metaclass is SysML::FlowUsage (8.3.16.3),
    // whose general classes are Flow, ActionUsage and ConnectorAsUsage — an ActionUsage,
    // yet the grammar lists it among the STRUCTURE usages, and the grammar decides the
    // membership.
    //
    // implied specialization: Flows::messages, and Flows::flows when it has end features
    // constraint: FlowUsage::checkFlowUsageSpecialization
    //     `specializesFromLibrary('Flows::messages')` (SysML 8.3.16.3, receipt 92bc5ec5)
    // constraint: FlowUsage::checkFlowUsageFlowSpecialization
    //     `ownedEndFeatures->notEmpty() implies specializesFromLibrary('Flows::flows')`
    //     Both are injections, so they belong in sv2-hir; this layer builds the tree only
    //     (ADR-0002). The `from … to …` ends are what make the second one bite.
    fn flow_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("flow");
        self.flow_declaration();
        self.definition_body();
        self.finish_node();
    }

    // production: FlowDeclaration@sysml
    //
    // FlowDeclaration : FlowUsage =
    //       UsageDeclaration ValuePart?
    //       ( 'of'  ownedRelationship += FlowPayloadFeatureMember )?
    //       ( 'from' ownedRelationship += FlowEndMember
    //         'to'   ownedRelationship += FlowEndMember )?
    //     | ownedRelationship += FlowEndMember 'to'
    //       ownedRelationship += FlowEndMember                    (SysML 8.2.2.16)
    //
    // The two alternatives can both open on a NAME: `flow f from a.b to c.d;` declares
    // `f`, and `flow a.b to c.d;` names an end. What separates them is the `to` — only
    // the second writes one directly after its first end, and the first writes `to` only
    // after `from`. So a whole flow end is looked past and the token after it decides.
    // The UsageDeclaration is built whenever the first alternative is taken, empty or
    // not, since its Identification may be empty — the corpus's `flow from …` is that.
    fn flow_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowDeclaration);
        let ends_first = self
            .flow_end_segments(0)
            .is_some_and(|(after, _)| self.nth_is_keyword(after, "to"));
        if ends_first {
            self.flow_end_member();
            self.expect_keyword("to");
            self.flow_end_member();
        } else {
            self.usage_declaration();
            if self.at_value_part() {
                self.value_part();
            }
            if self.at_keyword("of") {
                self.expect_keyword("of");
                self.flow_payload_feature_member();
            }
            if self.at_keyword("from") {
                self.expect_keyword("from");
                self.flow_end_member();
                self.expect_keyword("to");
                self.flow_end_member();
            }
        }
        self.finish_node();
    }

    /// The index just past a flow end written from the `n`th token, and how many
    /// `.`-separated segments it has, or `None` if no segment starts there.
    ///
    /// A flow end is `FlowEndSubsetting? FlowFeatureMember`, and every part of it is a
    /// `QualifiedName` followed by a `.` except the last (8.2.2.16, with deviation
    /// `FlowEndSubsetting`). Walked exactly as `flow_end` reads it: a `.` continues the
    /// end only when a NAME follows it.
    fn flow_end_segments(&self, n: usize) -> Option<(usize, usize)> {
        let mut after = self.skip_qualified_name(n)?;
        let mut segments = 1;
        while self.nth_is(after, SyntaxKind::Dot) && self.nth_is_name(after + 1) {
            after = self.skip_qualified_name(after + 1)?;
            segments += 1;
        }
        Some((after, segments))
    }

    // production: FlowEndMember
    //
    // FlowEndMember : EndFeatureMembership = ownedRelatedElement += FlowEnd
    //                                                            (SysML 8.2.2.16)
    //
    // Stated alike in KerML 8.2.5.9.2, so one shared grammar unit (ADR-0015).
    fn flow_end_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowEndMember);
        self.flow_end();
        self.finish_node();
    }

    // production: FlowEnd@sysml
    //
    // FlowEnd = ( ownedRelationship += FlowEndSubsetting )?
    //           ownedRelationship += FlowFeatureMember           (SysML 8.2.2.16)
    //
    // production: FlowEndSubsetting@sysml
    //
    // FlowEndSubsetting : ReferenceSubsetting =
    //       referencedFeature = [QualifiedName] '.'
    //     | referencedFeature = FeatureChainPrefix              (SysML 8.2.2.16)
    //
    // The '.' in the first alternative is deviation FlowEndSubsetting (follow_xtext,
    // adjudicated 2026-09-17): the clause omits it, nothing else in the clause could
    // consume it, and KerML's FlowEnd (8.2.5.9.2) writes `OwnedReferenceSubsetting '.'`.
    //
    // production: FeatureChainPrefix@sysml
    //
    // FeatureChainPrefix : Feature =
    //     ( ownedRelationship += OwnedFeatureChaining '.' )+
    //     ownedRelationship += OwnedFeatureChaining '.'          (SysML 8.2.2.16)
    //
    // Which of the three shapes an end takes is fixed by how many segments it has, so
    // the count is taken first and each shape built outright: one segment is the
    // feature alone, two are `[QualifiedName] '.'` and the feature, and three or more
    // put every segment but the last in a FeatureChainPrefix, whose `+` is what makes
    // its minimum two.
    fn flow_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowEnd);
        match self.flow_end_segments(0) {
            None => self.error_expected("a flow end"),
            Some((_, 1)) => self.flow_feature_member(),
            Some((_, 2)) => {
                self.start_node(SyntaxKind::FlowEndSubsetting);
                self.qualified_name();
                self.expect(SyntaxKind::Dot, "`.`");
                self.finish_node();
                self.flow_feature_member();
            }
            Some((_, segments)) => {
                self.start_node(SyntaxKind::FlowEndSubsetting);
                self.eat_trivia();
                self.start_node(SyntaxKind::FeatureChainPrefix);
                for _ in 1..segments {
                    self.owned_feature_chaining();
                    self.expect(SyntaxKind::Dot, "`.`");
                }
                self.finish_node();
                self.finish_node();
                self.flow_feature_member();
            }
        }
        self.finish_node();
    }

    // production: FlowFeatureMember
    //
    // FlowFeatureMember : FeatureMembership = ownedRelatedElement += FlowFeature
    //
    // production: FlowFeature
    //
    // FlowFeature : ReferenceUsage = ownedRelationship += FlowFeatureRedefinition
    //
    // production: FlowFeatureRedefinition
    //
    // FlowFeatureRedefinition : Redefinition = redefinedFeature = [QualifiedName]
    //                                                            (SysML 8.2.2.16)
    //
    // All three stated alike in KerML 8.2.5.9.2, so shared units — alike in SYNTAX: KerML
    // returns `FlowFeature : Feature` where SysML returns `ReferenceUsage`, and ADR-0015
    // shares a unit on its body, not its metaclass (derived unit FlowFeature.json says
    // so). Which metaclass is built is sv2-hir's question, per language. SysML's clause heads
    // the last one `FlowFeatureRefefinition` while its FlowFeature references it
    // correctly; deviation FlowFeatureRedefinition (follow_spec) reads the reference.
    fn flow_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowFeatureMember);
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowFeature);
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowFeatureRedefinition);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: FlowPayloadFeatureMember@sysml
    //
    // FlowPayloadFeatureMember : FeatureMembership =
    //     ownedRelatedElement += FlowPayloadFeature
    //
    // production: FlowPayloadFeature@sysml
    //
    // FlowPayloadFeature : PayloadFeature = PayloadFeature      (SysML 8.2.2.16)
    //
    // FlowPayloadFeature returns the PayloadFeature metaclass and reads the PayloadFeature
    // production, so it is a node over one child, as UsageBody is over DefinitionBody.
    fn flow_payload_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowPayloadFeatureMember);
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowPayloadFeature);
        self.payload_feature();
        self.finish_node();
        self.finish_node();
    }

    // production: PayloadFeature@sysml
    //
    // PayloadFeature : Feature =
    //       Identification? PayloadFeatureSpecializationPart ValuePart?
    //     | ownedRelationship += OwnedFeatureTyping
    //       ( ownedRelationship += OwnedMultiplicity )?
    //     | ownedRelationship += OwnedMultiplicity
    //       ownedRelationship += OwnedFeatureTyping             (SysML 8.2.2.16)
    //
    // Three alternatives, and the first two can open on the same NAME: `of fuel : Fuel`
    // names the payload and types it, `of Fuel` only types it. The first always goes on
    // to a FeatureSpecialization, possibly after a multiplicity, and the other two never
    // do; `payload_feature_is_declared` looks for one. The first's `Identification?` is
    // built whenever that alternative is taken, as Identification derives the empty
    // string anyway and one shape is simpler to consume than two.
    fn payload_feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadFeature);
        if self.payload_feature_is_declared() {
            self.identification();
            self.payload_feature_specialization_part();
            if self.at_value_part() {
                self.value_part();
            }
        } else if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
            self.owned_feature_typing();
        } else {
            self.owned_feature_typing();
            if self.at(SyntaxKind::LBracket) {
                self.owned_multiplicity();
            }
        }
        self.finish_node();
    }

    /// Whether the payload here is `PayloadFeature`'s first alternative.
    ///
    /// It is when a `FeatureSpecialization`, or `ordered`/`nonunique`, comes after at most
    /// a short name, one NAME and one bracketed multiplicity — which is everything
    /// `Identification? PayloadFeatureSpecializationPart` can put before its first
    /// `FeatureSpecialization`. The other two alternatives write a type where that would
    /// be, and a type is not a `FeatureSpecialization`.
    fn payload_feature_is_declared(&self) -> bool {
        let mut n = 0;
        if self.nth_is(n, SyntaxKind::Lt) {
            return true;
        }
        if self.nth_is_name(n) {
            n += 1;
        }
        if self.nth_is(n, SyntaxKind::LBracket) {
            match self.skip_bracketed(n) {
                Some(after) => n = after,
                None => return false,
            }
        }
        self.nth_at_feature_specialization(n)
            || self.nth_is_keyword(n, "ordered")
            || self.nth_is_keyword(n, "nonunique")
    }

    /// The index just past the `]` matching a `[` at the `n`th token, or `None` if it is
    /// not closed before the input ends.
    fn skip_bracketed(&self, n: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let mut at = n;
        loop {
            let kind = self.peek_nth(at)?.kind;
            if kind == SyntaxKind::LBracket {
                depth += 1;
            } else if kind == SyntaxKind::RBracket {
                depth = depth.saturating_sub(1);
            }
            at += 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }

    // production: PayloadFeatureSpecializationPart
    //
    // PayloadFeatureSpecializationPart : Feature =
    //       FeatureSpecialization+ MultiplicityPart?
    //       FeatureSpecialization*
    //     | MultiplicityPart FeatureSpecialization+            (KerML 8.2.5.9.2)
    //
    // KerML's clause, and the shared unit's rule. SysML 8.2.2.16 prints the first
    // alternative as `( -> FeatureSpecialization )+`: the pilot's `->` syntactic
    // predicate, carried into the specification text itself. It steers an LL parser and
    // does not change the language, so it is not ported — the decision recorded in the
    // derived unit PayloadFeatureSpecializationPart.json. The pinned Tier B′
    // transcription (SysML-textual-bnf.kebnf) already writes it without the `->`.
    //
    // FeatureSpecializationPart's shape with one difference: BOTH alternatives need a
    // FeatureSpecialization, where FeatureSpecializationPart's second admits a
    // multiplicity alone. So the loop is the same — any order, at most one
    // MultiplicityPart — and a part that read no FeatureSpecialization is reported.
    fn payload_feature_specialization_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadFeatureSpecializationPart);
        let mut multiplicity_taken = false;
        let mut specialized = false;
        loop {
            if self.at_feature_specialization() {
                self.feature_specialization();
                specialized = true;
            } else if !multiplicity_taken && self.at_multiplicity_part() {
                self.multiplicity_part();
                multiplicity_taken = true;
            } else {
                break;
            }
        }
        if !specialized {
            self.error_expected("a feature specialization");
        }
        self.finish_node();
    }

    // production: PerformActionUsage
    //
    // PerformActionUsage = OccurrenceUsagePrefix 'perform'
    //                      PerformActionUsageDeclaration ActionBody
    //                                                            (SysML 8.2.2.17.2)
    //
    // production: PerformActionUsageDeclaration
    //
    // PerformActionUsageDeclaration : PerformActionUsage =
    //     ( ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     | 'action' UsageDeclaration ) ValuePart?               (SysML 8.2.2.17.2)
    //
    // The metaclass is SysML::PerformActionUsage (8.3.17.14), both an ActionUsage and an
    // EventOccurrenceUsage. It performs an action rather than being one, and the two
    // alternatives are the two ways of saying which: by REFERENCE to an action declared
    // elsewhere, or by declaring one inline behind the `action` keyword.
    //
    // The alternatives are told apart on one token, before either can consume anything:
    // the second opens on the keyword `action` and the first on a QualifiedName, and a
    // keyword is not a name (SysML 8.2.2.1.2). The same shape RequirementConstraintUsage
    // has, one clause along.
    //
    // The by-reference alternative is why the four reference productions had to stop
    // claiming a chain they did not read: `perform providePower.generateTorque` is the
    // commonest `perform` in the corpus, and its target is an OwnedFeatureChain.
    fn perform_action_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PerformActionUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("perform");
        self.perform_action_usage_declaration();
        self.action_body();
        self.finish_node();
    }

    fn perform_action_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PerformActionUsageDeclaration);
        if self.at_keyword("action") {
            self.bump_as(keyword("action").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            if self.at_feature_specialization() {
                self.feature_specialization_part();
            }
        }
        if self.at_value_part() {
            self.value_part();
        }
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

    /// Whether a `ReturnParameterMember` starts here.
    ///
    /// `MemberPrefix? 'return'` (`SysML` 8.2.2.19). `return` is a pinned keyword and a
    /// keyword is not a name (`SysML` 8.2.2.1.2), so nothing else can open on it: it
    /// cannot be the bare name a `DefaultReferenceUsage` opens on, and it cannot be the
    /// first token of the trailing `ResultExpressionMember` either.
    fn at_return_parameter_member(&self) -> bool {
        self.nth_is_keyword(usize::from(self.at_visibility()), "return")
    }

    // production: ReturnParameterMember
    //
    // ReturnParameterMember : ReturnParameterMembership =
    //     MemberPrefix? 'return'
    //     ownedRelatedElement += UsageElement                     (SysML 8.2.2.19)
    //
    // The metaclass is KerML's ReturnParameterMembership (8.3.4.7.8), a
    // ParameterMembership. It owns its element through that membership rather than
    // through the body's ordinary member node, which is why — like SubjectMember and
    // RequirementConstraintMember — it is dispatched in `body_elements` and not in
    // `membership`.
    //
    // constraint: ReturnParameterMembership::validateReturnParameterMembershipOwningType
    //     `owningType.oclIsKindOf(Function) or owningType.oclIsKindOf(Expression)`
    //     (KerML 8.3.4.7.8). The grammar already reaches this production only from a
    //     CalculationBody, so the tree cannot violate it; it is still sv2-hir's to
    //     check rather than this layer's (ADR-0002).
    //
    // The clause also states that the ownedMemberParameter's DIRECTION MUST BE `out`
    // (KerML 8.3.4.7.8), and validateParameterMembershipParameterDirection (8.3.4.6.4)
    // is where that is enforced. It is a property the text does not write, so it is an
    // INJECTION and belongs in sv2-hir. Nothing here supplies it, and nothing here may:
    // writing `out` into the tree would break losslessness (invariant 1).
    //
    // `MemberPrefix?` — the `?` is redundant, as it is on ResultExpressionMember and
    // RequirementConstraintMember: MemberPrefix is itself `VisibilityIndicator?` and
    // already derives the empty string. Kept because the clause writes it.
    //
    // UsageElement is the FULL alternation (8.2.2.5.2) and only part of it is
    // implemented, so UsageElement is NOT marked for coverage — `usage_element` returns
    // whether it read one. This member IS marked, because its own shape is complete,
    // which is the same line NonFeatureMember and PackageMember draw.
    fn return_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ReturnParameterMember);
        self.member_prefix();
        self.expect_keyword("return");
        if !self.usage_element() {
            self.error_expected("a usage after `return`");
        }
        self.finish_node();
    }

    /// Whether an `InitialNodeMember` starts here.
    ///
    /// `first` alone does not say so. Three productions reachable from an action body
    /// open on it — this one, `SuccessionAsUsage` (`'first' ConnectorEndMember 'then'`,
    /// `SysML` 8.2.2.13.3) and `GuardedSuccession` (`'first' FeatureChainMember
    /// GuardExpressionMember 'then'`, 8.2.2.17.8) — and what separates them is what
    /// follows the name: only this one ends in `RelationshipBody`, which opens on `;` or
    /// `{` (8.2.2.2). So the whole `QualifiedName` is looked past, by the same rule
    /// `qualified_name` reads it with, and the token after it decides. Committing on
    /// `first` would build an `InitialNodeMember` out of `first a then b;` and report the
    /// rest, which is a tree for a production the text does not contain.
    ///
    /// `GuardedSuccession` is now implemented and is dispatched before this, so a `first`
    /// this declines because an `if` follows the name is READ, not reported.
    /// `SuccessionAsUsage` is not, so `first a then b;` is still recovered over and
    /// reported — rejected by absence, not by rule.
    fn at_initial_node_member(&self) -> bool {
        let first = usize::from(self.at_visibility());
        if !self.nth_is_keyword(first, "first") {
            return false;
        }
        let is = |n: usize, kind: SyntaxKind| self.peek_nth(n).is_some_and(|t| t.kind == kind);
        let named = |n: usize| self.peek_nth(n).is_some_and(|t| self.is_name(t));
        // QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME   (KerML 8.2.3.4.1)
        let mut n = first + 1;
        if is(n, SyntaxKind::Dollar) && is(n + 1, SyntaxKind::ColonColon) {
            n += 2;
        }
        if !named(n) {
            return false;
        }
        n += 1;
        // A `::` is the name's only when a NAME follows it, as in `qualified_name`.
        while is(n, SyntaxKind::ColonColon) && named(n + 1) {
            n += 2;
        }
        is(n, SyntaxKind::Semicolon) || is(n, SyntaxKind::LBrace)
    }

    // production: InitialNodeMember
    //
    // InitialNodeMember : FeatureMembership =
    //     MemberPrefix 'first' memberFeature = [QualifiedName]
    //     RelationshipBody                                        (SysML 8.2.2.17.1)
    //
    // `first X;` names the SOURCE of a succession separately from its target, which a
    // following `then` supplies (SysML 7.17.4); `first start;` — the start snapshot every
    // action inherits from Actions::Action — is 15 of the corpus's 16. The succession it
    // opens is ActionTargetSuccessionMember, which is unimplemented, so today the member
    // stands alone: every corpus file that writes it goes on to `then`.
    //
    // The memberFeature is a REFERENCE, `[QualifiedName]`, not an ownedRelatedElement:
    // the member owns no element, which is why the tree holds a QualifiedName and no
    // usage node. The clause states the metaclass as FeatureMembership; the Pilot returns
    // SysML::Membership and assigns memberElement — the same syntax, and the verified
    // reference unit (InitialNodeMember@sysml) follows the clause. Like
    // ReturnParameterMember it is its own membership, so it is dispatched in
    // `body_elements` and not in `membership`.
    //
    // The RelationshipBody is not optional, so `first start` with no `;` is not this
    // production: `at_initial_node_member` declines it and the recovery reports it.
    fn initial_node_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InitialNodeMember);
        self.member_prefix();
        self.expect_keyword("first");
        self.qualified_name();
        self.relationship_body();
        self.finish_node();
    }

    /// `ActionBodyItem`'s second alternative, whole:
    /// `InitialNodeMember ActionTargetSuccessionMember*` (`SysML` 8.2.2.17.1).
    ///
    /// ONE item, so the `then X;` members are read here, directly after the `first`,
    /// and are not an arm of `body_elements`: the BNF admits them only as this suffix
    /// (or an `ActionBehaviorMember`'s, unimplemented). An attribute between the two
    /// ends the item and the `then` after it is reported —
    /// tests/rejection/target-succession-member-does-not-skip-a-non-behavior-item.sysml.
    /// Which element a `then` connects FROM is 7.17.4's "nearest occurrence lexically
    /// previous to the then" (wiki receipt 339ef468), a question for resolution, not
    /// for the parser. No node of its own: `ActionBodyItem` is not one.
    /// Whether a `GuardedSuccessionMember` starts here.
    ///
    /// `( 'succession' UsageDeclaration )? 'first' FeatureChainMember
    /// GuardExpressionMember` (`SysML` 8.2.2.17.8), so the `if` after the source name is
    /// what says so. `at_initial_node_member` requires a `;` or `{` in that same place, and
    /// `SuccessionAsUsage` a `then` (8.2.2.13.3), so the three `first` productions are
    /// disjoint on one token and none of them commits before reaching it.
    ///
    /// The optional declaration is looked past by scanning to the `first`, bounded by the
    /// tokens no `UsageDeclaration` contains: a `;`, a body brace, or the end of input.
    fn at_guarded_succession_member(&self) -> bool {
        let mut n = usize::from(self.at_visibility());
        if self.nth_is_keyword(n, "succession") {
            match self.scan_for_keyword(n + 1, "first") {
                Some(first) => n = first,
                None => return false,
            }
        }
        if !self.nth_is_keyword(n, "first") {
            return false;
        }
        // FeatureChainMember: a QualifiedName, and then OwnedFeatureChain's further links.
        let Some(mut n) = self.skip_qualified_name(n + 1) else {
            return false;
        };
        while self.nth_is(n, SyntaxKind::Dot) && self.nth_is_name(n + 1) {
            let Some(next) = self.skip_qualified_name(n + 1) else {
                return false;
            };
            n = next;
        }
        self.nth_is_keyword(n, "if")
    }

    // production: GuardedSuccessionMember@sysml
    //
    // GuardedSuccessionMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += GuardedSuccession     (SysML 8.2.2.17.1)
    //
    // ActionBodyItem's FOURTH alternative (receipt 4e3ffb79). It carries no
    // ActionTargetSuccessionMember* after it, unlike the SECOND and THIRD, which both do;
    // the first, NonBehaviorBodyItem, carries none either, so what is particular here is
    // that a succession takes no suffix, not that only this alternative lacks one. This
    // is therefore an item, never a suffix, and nothing may suffix it. Owns its element through a membership of its own,
    // as InitialNodeMember does, so it is dispatched from `body_elements` rather than
    // through `membership`.
    fn guarded_succession_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::GuardedSuccessionMember);
        self.member_prefix();
        self.guarded_succession();
        self.finish_node();
    }

    // production: GuardedSuccession@sysml
    //
    // GuardedSuccession : TransitionUsage =
    //     ( 'succession' UsageDeclaration )?
    //     'first' ownedRelationship += FeatureChainMember
    //     ownedRelationship += GuardExpressionMember
    //     'then' ownedRelationship += TransitionSuccessionMember
    //     UsageBody                                                 (SysML 8.2.2.17.8)
    //
    // The succession that writes its OWN source (receipt e5f3ae62): where a target
    // succession leaves the source to the item before it (7.17.4), this names it after
    // `first`. A TransitionUsage (8.3.18.9, receipt a6f32577) over the same
    // GuardExpressionMember and TransitionSuccessionMember the guarded target uses, which
    // is why only the source and the optional declaration are new here.
    //
    // implied specialization: Actions::Action::decisionTransitions, as the guarded target
    //     succession carries — sv2-hir's to inject (ADR-0002).
    fn guarded_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::GuardedSuccession);
        if self.at_keyword("succession") {
            self.bump_as(keyword("succession").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        }
        self.expect_keyword("first");
        self.sysml_feature_chain_member();
        self.guard_expression_member();
        self.expect_keyword("then");
        self.transition_succession_member();
        self.usage_body();
        self.finish_node();
    }

    // production: FeatureChainMember@sysml
    //
    // FeatureChainMember : Membership =
    //     memberElement = [QualifiedName]
    //     | ownedRelationship += OwnedFeatureChainMember             (SysML 8.2.2.17.5)
    //
    // production: OwnedFeatureChainMember@sysml
    //
    // OwnedFeatureChainMember : OwningMembership =
    //     ownedRelatedElement += OwnedFeatureChain                   (SysML 8.2.2.17.5)
    //
    // Stated at 8.2.2.17.5, Assignment Action Usages (receipt dfe847fa), and used by four
    // productions elsewhere — GuardedSuccession is the first of them this parser reads.
    // NOT KerML's FeatureChainMember, whose first alternative is a FeatureReferenceMember
    // (8.2.5.8.2); see `kerml_feature_chain_member`.
    //
    // OwnedFeatureChain's `( '.' link )+` needs at least two links (8.2.2.6.5), so one
    // name is the reference alternative and two or more the owned one. The alternative is
    // chosen by looking past the name rather than by building one and repairing it: the
    // member node differs between the two, and a checkpoint cannot un-own an element.
    //
    // The deviation register carries ONE entry for OwnedFeatureChainMember, bare-named as
    // every entry is, and it is evidenced at KerML 8.2.5.8.2 — not at this SysML clause:
    // spec_only, resolved follow_spec, because the Pilot has no such rule and the
    // specification is normative. The unit implemented here is the SysML one, verified in
    // its own right at 8.2.2.17.5 (receipt dfe847fa), and the register's reasoning covers
    // it as far as it goes: no pinned file exercises the production, which is why
    // a_guarded_successions_source_takes_either_alternative constructs the case from the
    // production rather than from the corpus.
    fn sysml_feature_chain_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureChainMember);
        if self.at_sysml_owned_feature_chain() {
            self.start_node(SyntaxKind::OwnedFeatureChainMember);
            let start = self.builder.checkpoint();
            self.qualified_name();
            self.owned_feature_chain(start);
            self.finish_node();
        } else {
            self.qualified_name();
        }
        self.finish_node();
    }

    /// Whether the name starting here is followed by a `.` and another name, making it an
    /// `OwnedFeatureChain` rather than a bare `QualifiedName`.
    fn at_sysml_owned_feature_chain(&self) -> bool {
        self.skip_qualified_name(0)
            .is_some_and(|after| self.nth_is(after, SyntaxKind::Dot) && self.nth_is_name(after + 1))
    }

    fn initial_node_item(&mut self) {
        self.initial_node_member();
        while self.at_action_target_succession_member() {
            self.action_target_succession_member();
        }
    }

    /// Whether an `ActionTargetSuccessionMember` in its `TargetSuccession` form starts
    /// here.
    ///
    /// `then NAME` does not say so. `SourceSuccessionMember ActionBehaviorMember`, the
    /// third `ActionBodyItem` alternative (`SysML` 8.2.2.17.1), also opens on `then`, and
    /// one of its behaviours, `SendNode`, opens on an optional `ActionUsageDeclaration`
    /// — a bare `Identification` — before `send` (8.2.2.17.4). So `then s send x;` has
    /// a NAME after its `then` and is not this production. What separates them is the
    /// end: only this one closes the `ConnectorEnd` with `UsageBody`, which opens on `;`
    /// or `{` (8.2.2.6.1). The whole `ConnectorEnd` is therefore looked past, walked as
    /// `connector_end` reads it, and the token after it decides.
    ///
    /// `then fork;` and `then action a;` are declined sooner: `fork` and `action` are
    /// reserved, so not a NAME — the second is `SourceSuccessionMember`'s, and the first
    /// an `ActionNode`'s. `GuardedTargetSuccession` (`if`), `DefaultTargetSuccession`
    /// (`else`) and a `[` AFTER the `then` — the `ConnectorEnd`'s
    /// `OwnedCrossMultiplicityMember` — are declined for the same reason and are
    /// unimplemented, so each is left to the enclosing body's recovery and reported. A
    /// `[` BEFORE the `then` is the `SourceEnd`'s multiplicity and is looked past.
    fn at_action_target_succession_member(&self) -> bool {
        let first = usize::from(self.at_visibility());
        self.at_guarded_target_succession(first)
            // DefaultTargetSuccession = 'else' TransitionSuccessionMember (8.2.2.17.8).
            // One keyword decides it: `else` is reserved, and the only other production
            // that writes one is KerML's ConditionalExpression, whose `else` stands
            // INSIDE the expression (`if c ? a else b`, 8.2.5.8.1) and so is never at an
            // item position. No UsageBody lookahead, as the guarded form has none: the
            // member is read and UsageBody reports its own absence.
            || self.nth_is_keyword(first, "else")
            || self.at_target_succession(first)
    }

    /// Whether a `GuardedTargetSuccession` starts at the `n`th meaningful token.
    ///
    /// `GuardExpressionMember 'then' TransitionSuccessionMember` (`SysML` 8.2.2.17.8), so
    /// `if`, an expression, and a `then`. The `then` is what has to be found, because
    /// `IfNode` opens on `if` as well — `ActionNodePrefix 'if' ExpressionParameterMember
    /// ActionBodyParameterMember` (8.2.2.17.7), an `ActionNode` that is unimplemented —
    /// and so does `KerML`'s `ConditionalExpression`, `'if' Expression '?' Expression
    /// 'else' Expression` (8.2.5.8.1), which a calculation body may write as its result.
    ///
    /// The expression between the two keywords is scanned rather than parsed: no
    /// implemented expression contains a `then`, and none reaches a `;`, a `{` or a `}`
    /// without ending, so the first of those four tokens decides. `if i < 0 { }` is an
    /// `IfNode` and reported; `if x ? 1 else 2 }` is the result expression and left to
    /// the body.
    fn at_guarded_target_succession(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "if") && self.scan_for_keyword(n + 1, "then").is_some()
    }

    /// The index of the next `word` at or after the `n`th token, if one is reached before
    /// the statement ends.
    ///
    /// Bounded by the tokens that end a statement — `;` and either brace — and by the end
    /// of the input, so a truncated `if x` declines rather than scanning for ever
    /// (invariant 3). This is a token scan and not a parse, which is sound only because
    /// the words it looks for are RESERVED: `then` and `first` cannot be names
    /// (`SysML` 8.2.2.1.2), and no implemented production between them writes a `;` or a
    /// brace and then continues.
    fn scan_for_keyword(&self, n: usize, word: &str) -> Option<usize> {
        let mut n = n;
        while !self.nth_is_keyword(n, word) {
            if self.peek_nth(n).is_none()
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::LBrace)
                || self.nth_is(n, SyntaxKind::RBrace)
            {
                return None;
            }
            n += 1;
        }
        Some(n)
    }

    /// Whether an `ActionTargetSuccessionMember` in its `TargetSuccession` form starts at
    /// the `n`th meaningful token.
    fn at_target_succession(&self, n: usize) -> bool {
        let mut first = n;
        // TargetSuccession's SourceEnd, `OwnedMultiplicity?`, stands BEFORE its `then`
        // (8.2.2.17.8, 8.2.2.9.3): `first start; [1] then a;`.
        if self.nth_is(first, SyntaxKind::LBracket) {
            let Some(after) = self.skip_bracketed(first) else {
                return false;
            };
            first = after;
        }
        if !self.nth_is_keyword(first, "then") {
            return false;
        }
        self.skip_connector_end(first + 1).is_some_and(|n| {
            self.nth_is(n, SyntaxKind::Semicolon) || self.nth_is(n, SyntaxKind::LBrace)
        })
    }

    /// The index just past a `ConnectorEnd` written at the `n`th meaningful token, walked
    /// as `connector_end` reads it, or `None` if one is not written there.
    ///
    /// `( NAME REFERENCES )? OwnedReferenceSubsetting` (`SysML` 8.2.2.13.1), where
    /// `REFERENCES = '::>' | 'references'` (8.2.2.1.2) and the subsetting is a
    /// `QualifiedName` followed by `OwnedFeatureChain`'s further links (8.2.2.6.5). The
    /// leading `OwnedCrossMultiplicityMember` is unimplemented and not looked past, so an
    /// end that writes one is declined here and reported by the caller's recovery.
    fn skip_connector_end(&self, n: usize) -> Option<usize> {
        let mut n = n;
        if self.nth_is_name(n)
            && (self.nth_is(n + 1, SyntaxKind::ColonColonGt)
                || self.nth_is_keyword(n + 1, "references"))
        {
            n += 2;
        }
        let mut n = self.skip_qualified_name(n)?;
        while self.nth_is(n, SyntaxKind::Dot) && self.nth_is_name(n + 1) {
            n = self.skip_qualified_name(n + 1)?;
        }
        Some(n)
    }

    // production: ActionTargetSuccessionMember
    //
    // ActionTargetSuccessionMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += ActionTargetSuccession  (SysML 8.2.2.17.1)
    //
    // `then X;`: the TARGET of a succession written apart from its source, which the
    // InitialNodeMember before it names (SysML 7.17.4). Like InitialNodeMember it is a
    // membership of its own, so it is dispatched from `body_elements` and not through
    // `membership`, and only as the suffix of the item that precedes it.
    //
    // Marked although ActionTargetSuccession is not: this production's own body is read
    // in full, as UsageBody is marked over a partial DefinitionBodyItem.
    fn action_target_succession_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionTargetSuccessionMember);
        self.member_prefix();
        self.action_target_succession();
        self.finish_node();
    }

    // ActionTargetSuccession : Usage =
    //     ( TargetSuccession | GuardedTargetSuccession | DefaultTargetSuccession )
    //     UsageBody                                                (SysML 8.2.2.17.8)
    //
    // production: ActionTargetSuccession@sysml
    //
    // Marked now that all THREE alternatives are read. It was unmarked while
    // DefaultTargetSuccession was absent, which is the convention: a production whose
    // alternatives are partly done is a tracked gap, not a claim.
    //
    // The three are told apart on one token each, and none of them can be a name: `if`
    // opens the guarded form, `else` the default, and anything else is the plain one,
    // whose own recogniser has already looked past its ConnectorEnd to the UsageBody
    // before this is called.
    fn action_target_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionTargetSuccession);
        if self.at_keyword("if") {
            self.guarded_target_succession();
        } else if self.at_keyword("else") {
            self.default_target_succession();
        } else {
            self.target_succession();
        }
        self.usage_body();
        self.finish_node();
    }

    // production: DefaultTargetSuccession@sysml
    //
    // DefaultTargetSuccession : TransitionUsage =
    //     'else' ownedRelationship += TransitionSuccessionMember    (SysML 8.2.2.17.8)
    //
    // The branch taken when no guard before it held (8.2.2.17.8, receipt e5f3ae62). A
    // TransitionUsage as the guarded form is (8.3.18.9, receipt a6f32577), over the same
    // TransitionSuccessionMember, and it owns no GuardExpressionMember at all: the `else`
    // is the whole of the condition. WHICH guards it defaults over is resolution's to
    // find, as a succession's unwritten source is (7.17.4, receipt 339ef468) — the
    // grammar does not require one to precede it, and `first start; else b;` parses.
    fn default_target_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefaultTargetSuccession);
        self.expect_keyword("else");
        self.transition_succession_member();
        self.finish_node();
    }

    // production: GuardedTargetSuccession@sysml
    //
    // GuardedTargetSuccession : TransitionUsage =
    //     ownedRelationship += GuardExpressionMember
    //     'then' ownedRelationship += TransitionSuccessionMember    (SysML 8.2.2.17.8)
    //
    // A guard where TargetSuccession writes its source end (8.2.2.17.8, receipt
    // e5f3ae62). The metaclass is a TransitionUsage (8.3.18.9, receipt a6f32577) and NOT
    // a SuccessionAsUsage, which is why the target hangs off a TransitionSuccession here
    // and off a ConnectorEndMember there: a TransitionUsage owns the succession it
    // asserts as a feature, and derives its own source from where it stands (7.17.4).
    //
    // implied specialization: Actions::Action::decisionTransitions, for the guarded
    //     successions after a DecisionNode
    // constraint: TransitionUsage::checkTransitionUsageSpecialization and its siblings —
    //     injections, so sv2-hir's (ADR-0002). This layer builds the tree only.
    fn guarded_target_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::GuardedTargetSuccession);
        self.guard_expression_member();
        self.expect_keyword("then");
        self.transition_succession_member();
        self.finish_node();
    }

    // production: GuardExpressionMember@sysml
    //
    // GuardExpressionMember : TransitionFeatureMembership =
    //     'if' { kind = 'guard' }
    //     ownedRelatedElement += OwnedExpression                    (SysML 8.2.2.18.3)
    //
    // `{ kind = 'guard' }` assigns a TransitionFeatureMembership's kind (8.3.18.8,
    // receipt 7818cc5c) and contributes no token: the `if` is what says the feature is a
    // guard rather than a trigger or an effect.
    fn guard_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::GuardExpressionMember);
        self.expect_keyword("if");
        self.owned_expression();
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
        self.start_node(SyntaxKind::TransitionSuccession);
        self.start_node(SyntaxKind::EmptyEndMember);
        self.start_node(SyntaxKind::EmptyFeature);
        self.finish_node();
        self.finish_node();
        self.connector_end_member();
        self.finish_node();
        self.finish_node();
    }

    // production: TargetSuccession
    //
    // TargetSuccession : SuccessionAsUsage =
    //     ownedRelationship += SourceEndMember
    //     'then' ownedRelationship += ConnectorEndMember            (SysML 8.2.2.17.8)
    //
    // The source end comes BEFORE the `then` and is written empty — `SourceEnd` is only
    // `OwnedMultiplicity?` — because the source is not named here at all: it is the
    // preceding InitialNodeMember's feature (7.17.4), connected in resolution.
    fn target_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TargetSuccession);
        self.source_end_member();
        self.expect_keyword("then");
        self.connector_end_member();
        self.finish_node();
    }

    // production: SourceEndMember
    //
    // SourceEndMember : EndFeatureMembership =
    //     ownedRelatedElement += SourceEnd                          (SysML 8.2.2.9.3)
    //
    // production: SourceEnd@sysml
    //
    // SourceEnd : ReferenceUsage =
    //     ( ownedRelationship += OwnedMultiplicity )?               (SysML 8.2.2.9.3)
    //
    // Usually built from no tokens — the one node shape in this parser with nothing in
    // it but what MemberPrefix already has. Trivia is NOT eaten first: an empty node that
    // swallowed the whitespace before the `then` would put it inside an end the author
    // never wrote.
    //
    // The multiplicity is read wherever it is written, and the two callers put it on
    // opposite sides of their `then`: TargetSuccession writes `SourceEndMember 'then'`,
    // so `first start; [1] then a;` has it BEFORE, and SourceSuccessionMember writes
    // `'then' SourceSuccession`, so `then [1] action a;` has it AFTER. Each caller's
    // recogniser looks past a `[…]` in its position, which is what makes both
    // alternatives of this production reachable and so markable.
    fn source_end_member(&mut self) {
        self.start_node(SyntaxKind::SourceEndMember);
        self.start_node(SyntaxKind::SourceEnd);
        if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
        }
        self.finish_node();
        self.finish_node();
    }

    /// Whether a `SourceSuccessionMember` starts here, before the occurrence usage it
    /// must precede.
    ///
    /// `then`, the `SourceEnd`'s optional multiplicity, and then the MEMBER that the
    /// item production pairs it with — `OccurrenceUsageMember`, `StructureUsageMember`
    /// or `ActionBehaviorMember` (8.2.2.6.1, 8.2.2.17.1), each `MemberPrefix` and an
    /// occurrence usage. A `then` before anything else is not this: before a NAME and a
    /// `UsageBody` it is a target succession, and before a non-occurrence usage or a
    /// definition no item production has it at all.
    ///
    /// Before a control node it is this only where `ActionBodyItem` is an item
    /// production, which is why the body is asked: `DefinitionBodyItem`'s `then`
    /// prefixes an `OccurrenceUsageMember`, and an `ActionNode` is not one (8.2.2.6.1).
    fn at_source_succession_member(&self, body: Body) -> bool {
        if !self.nth_is_keyword(0, "then") {
            return false;
        }
        let mut n = 1;
        if self.nth_is(n, SyntaxKind::LBracket) {
            let Some(after) = self.skip_bracketed(n) else {
                return false;
            };
            n = after;
        }
        n += usize::from(VISIBILITY.iter().any(|word| self.nth_is_keyword(n, word)));
        self.at_action_usage(n)
            || self.at_perform_action_usage(n)
            || self.at_assert_constraint_usage(n)
            || self.at_constraint_usage(n)
            || self.at_calculation_usage(n)
            || self.at_flow_usage(n)
            || self
                .at_simple_usage(n)
                .is_some_and(|usage| usage.class != UsageClass::NonOccurrence)
            || (body.admits_action_body_item() && self.at_control_node(n).is_some())
    }

    /// Which `ControlNode` starts at the `n`th meaningful token, if one does.
    ///
    /// A `ControlNodePrefix` looked past, then one of the four keywords. The prefix is
    /// `RefPrefix 'individual'? PortionKind?` (`SysML` 8.2.2.17.3) — `RefPrefix`, not
    /// `BasicUsagePrefix`, so a `ref` is not looked past and `ref merge m;` is reported.
    /// `UsageExtensionKeyword*` is not looked past either: it is unimplemented, as it is
    /// on `OccurrenceUsagePrefix`.
    fn at_control_node(&self, n: usize) -> Option<(&'static str, SyntaxKind)> {
        let mut n = self.skip_ref_prefix(n);
        n += usize::from(self.nth_is_keyword(n, "individual"));
        n += usize::from(self.nth_is_keyword(n, "snapshot") || self.nth_is_keyword(n, "timeslice"));
        CONTROL_NODES
            .into_iter()
            .find(|(word, _)| self.nth_is_keyword(n, word))
    }

    // production: ControlNode@sysml
    //
    // ControlNode = MergeNode | DecisionNode | JoinNode | ForkNode   (SysML 8.2.2.17.3)
    //
    // production: MergeNode@sysml
    // production: DecisionNode@sysml
    // production: JoinNode@sysml
    // production: ForkNode@sysml
    //
    // MergeNode = ControlNodePrefix isComposite ?= 'merge' UsageDeclaration ActionBody
    //
    // and the other three the same over `decide`, `join` and `fork` (8.2.2.17.3, receipt
    // 08ce2499). One method reads all four, as `simple_usage` reads the seven, because
    // they differ in nothing the parser decides; each builds its own node so the tree
    // says which metaclass it is. ControlNode itself gets no node — an alternation, like
    // UsageElement — and is marked because every alternative is read.
    //
    // `isComposite ?= 'merge'` assigns a property from the keyword and adds no token:
    // validateControlNodeIsComposite requires a ControlNode to be composite (8.3.17.6,
    // receipt 695df335), and the keyword is how the text says so.
    //
    // The metaclasses are MergeNode (8.3.17.13, receipt e6e46c06), DecisionNode
    // (8.3.17.7, receipt 630e3433), JoinNode (8.3.17.11, receipt 48153851) and ForkNode
    // (8.3.17.8, receipt 58ebdb55), each a ControlNode, an ActionUsage (8.3.17.6).
    //
    // implied specialization: Actions::Action::merges, ::decisions, ::join, ::forks
    // constraint: MergeNode::checkMergeNodeSpecialization and its three siblings
    //     `specializesFromLibrary('Actions::Action::merges')` and so on. Injections
    //     belong in sv2-hir; this layer builds the tree only (ADR-0002).
    fn control_node(&mut self, node: SyntaxKind, word: &str) {
        self.eat_trivia();
        self.start_node(node);
        self.control_node_prefix();
        self.expect_keyword(word);
        self.usage_declaration();
        self.action_body();
        self.finish_node();
    }

    // ControlNodePrefix : OccurrenceUsage =
    //     RefPrefix ( isIndividual ?= 'individual' )?
    //     ( portionKind = PortionKind { isPortion = true } )?
    //     UsageExtensionKeyword*                                   (SysML 8.2.2.17.3)
    //
    // NOT marked for coverage: UsageExtensionKeyword (`#` prefix metadata) is
    // unimplemented, as on OccurrenceUsagePrefix. The clause's `'individual` is missing
    // its closing quote; deviation ControlNodePrefix (spec_only, follow_spec, SYSML21-400)
    // closes it, and the keyword read here is that repaired one. The node is built even
    // when every slot is empty, as OccurrenceUsagePrefix's is.
    fn control_node_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ControlNodePrefix);
        self.ref_prefix();
        self.eat_optional_keyword("individual");
        if self.at_keyword("snapshot") || self.at_keyword("timeslice") {
            self.portion_kind();
        }
        self.finish_node();
    }

    // production: SourceSuccessionMember@sysml
    //
    // SourceSuccessionMember : FeatureMembership =
    //     'then' ownedRelatedElement += SourceSuccession           (SysML 8.2.2.9.3)
    //
    // production: SourceSuccession@sysml
    //
    // SourceSuccession : SuccessionAsUsage =
    //     ownedRelationship += SourceEndMember                     (SysML 8.2.2.9.3)
    //
    // The TARGET's half of a succession whose target is the usage after it and whose
    // source is not written: 7.17.4 makes it the nearest occurrence lexically before the
    // `then` (receipt 339ef468), which is resolution's to find.
    fn source_succession_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SourceSuccessionMember);
        self.expect_keyword("then");
        self.start_node(SyntaxKind::SourceSuccession);
        self.source_end_member();
        self.finish_node();
        self.finish_node();
    }

    // production: ConnectorEndMember
    //
    // ConnectorEndMember : EndFeatureMembership =
    //     ownedRelatedElement += ConnectorEnd                       (SysML 8.2.2.13.1)
    //
    // KerML states the same production (8.2.5.5.1) with the same body, differing only in
    // the metaclass it returns, so ADR-0015 makes it ONE shared grammar unit and this
    // marker claims that unit. ConnectorEnd is shared the same way.
    fn connector_end_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConnectorEndMember);
        self.connector_end();
        self.finish_node();
    }

    // ConnectorEnd : ReferenceUsage =
    //     ( ownedRelationship += OwnedCrossMultiplicityMember )?
    //     ( declaredName = NAME REFERENCES )?
    //     ownedRelationship += OwnedReferenceSubsetting             (SysML 8.2.2.13.1)
    //
    // NOT marked: OwnedCrossMultiplicityMember is unimplemented, so a leading `[` is not
    // read. The other two parts are, and `at_action_target_succession_member` walks them
    // in this order.
    fn connector_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConnectorEnd);
        if self.at_name()
            && (self.nth_is(1, SyntaxKind::ColonColonGt) || self.nth_is_keyword(1, "references"))
        {
            self.bump();
            self.terminal(
                SyntaxKind::ColonColonGt,
                "references",
                "`::>` or `references`",
            );
        }
        self.owned_reference_subsetting();
        self.finish_node();
    }

    /// Whether a `SuccessionAsUsage` starts at the `n`th meaningful token.
    ///
    /// `UsagePrefix`, then either `first` or `succession` and a `UsageDeclaration` up to
    /// the `first`; then the source `ConnectorEnd`, and a `then` directly after it. The
    /// `then` is what decides: two other productions reachable from an action body open
    /// on `first` — `InitialNodeMember`, whose name is followed by `;` or `{`, and
    /// `GuardedSuccession`, whose source is followed by `if` (`SysML` 8.2.2.17.1,
    /// 8.2.2.17.8) — and all three are disjoint on that one token, so none commits before
    /// reaching it. `GuardedSuccession` shares the `succession` declaration as well, and
    /// is declined here for the same reason.
    ///
    /// The declaration is scanned rather than parsed, by `scan_for_keyword`, because
    /// `first` is reserved (8.2.2.1.2) and a `UsageDeclaration` writes no `;` or brace.
    /// `succession flow`, a `SuccessionFlowUsage` (8.2.2.16), writes no `first` at all, so
    /// the scan declines it at its `;`.
    ///
    /// The prefix skipped is `UsagePrefix`, which is what `succession_as_usage` reads, and
    /// NOT `OccurrenceUsagePrefix`: a recogniser that looked past `snapshot` would accept
    /// a member the parser then cannot consume, and the body loop would ask again for
    /// ever (invariant 3).
    fn at_succession_as_usage(&self, n: usize) -> bool {
        let mut n = self.skip_basic_usage_prefix(n);
        if self.nth_is_keyword(n, "succession") {
            match self.scan_for_keyword(n + 1, "first") {
                Some(first) => n = first,
                None => return false,
            }
        }
        self.nth_is_keyword(n, "first")
            && self
                .skip_connector_end(n + 1)
                .is_some_and(|after| self.nth_is_keyword(after, "then"))
    }

    // production: SuccessionAsUsage@sysml
    //
    // SuccessionAsUsage =
    //     UsagePrefix ( 'succession' UsageDeclaration )?
    //     'first' ownedRelationship += ConnectorEndMember
    //     'then' ownedRelationship += ConnectorEndMember
    //     UsageBody                                                 (SysML 8.2.2.13.3)
    //
    // A succession declared as a usage, naming both of its ends (receipt b9e0de2c). The
    // metaclass is SuccessionAsUsage (8.3.13.6, receipt 2d6e6f52), both a ConnectorAsUsage
    // and a KerML Succession. "A succession is not a kind of occurrence usage", so it
    // takes UsagePrefix and not OccurrenceUsagePrefix, and "if the declaration part is
    // empty, then the keyword succession may be omitted" (7.13.5, receipt 2abd302c) —
    // which is what the corpus's `first a then b;` is. The corpus also writes the keyword
    // over an EMPTY declaration, `succession first a then b;` (AHFSequences.sysml), which
    // the production admits because Identification is fully optional.
    //
    // Marked although ConnectorEnd is not: this production's own body is read in full,
    // as ActionTargetSuccessionMember is marked over the same ConnectorEndMember. What
    // ConnectorEnd lacks — its OwnedCrossMultiplicityMember, `first [1] a then b;` — is
    // held by tests/rejection/connector-end-cross-multiplicity-is-not-implemented.sysml.
    //
    // implied specialization: Occurrences::happensBeforeLinks
    // constraint: Succession::checkSuccessionSpecialization, which "requires that a
    //     SuccessionAsUsage specialize the KerML Feature Occurrences::happensBeforeLinks"
    //     (SysML 8.4.9.4, receipt 86e83273). An injection, so sv2-hir's; this layer builds
    //     the tree only (ADR-0002).
    fn succession_as_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuccessionAsUsage);
        self.usage_prefix();
        if self.at_keyword("succession") {
            self.bump_as(keyword("succession").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        }
        self.expect_keyword("first");
        self.connector_end_member();
        self.expect_keyword("then");
        self.connector_end_member();
        self.usage_body();
        self.finish_node();
    }

    /// Whether a `BindingConnectorAsUsage` starts at the `n`th meaningful token.
    ///
    /// `UsagePrefix`, then `binding` or `bind`. Both are reserved (`SysML` 8.2.2.1.2) and
    /// open no other production, so the keyword decides on its own, with none of the
    /// lookahead `at_succession_as_usage` needs to tell `first` apart: a binding missing
    /// its `=` or an end is read, and reported where the part is missing.
    ///
    /// The prefix skipped is `UsagePrefix`, which is what `binding_connector_as_usage`
    /// reads, and NOT `OccurrenceUsagePrefix`, for the reason `at_succession_as_usage`
    /// gives: a recogniser that looked past `snapshot` would accept a member the parser
    /// then cannot consume.
    fn at_binding_connector_as_usage(&self, n: usize) -> bool {
        let n = self.skip_basic_usage_prefix(n);
        self.nth_is_keyword(n, "binding") || self.nth_is_keyword(n, "bind")
    }

    // production: BindingConnectorAsUsage@sysml
    //
    // BindingConnectorAsUsage =
    //     UsagePrefix ( 'binding' UsageDeclaration )?
    //     'bind' ownedRelationship += ConnectorEndMember
    //     '=' ownedRelationship += ConnectorEndMember
    //     UsageBody                                                 (SysML 8.2.2.13.2)
    //
    // A binding declared as a usage, naming its two related features (receipt
    // 6c24121f). The metaclass is BindingConnectorAsUsage (8.3.13.2, receipt 9cf9f357),
    // both a ConnectorAsUsage and a KerML BindingConnector. "A binding is not a kind of
    // occurrence usage", so it takes UsagePrefix and not OccurrenceUsagePrefix, and "if
    // the declaration part is empty, then the keyword binding may be omitted" (7.13.3,
    // receipt 6db87b41) — which is what the corpus's `bind a = b;` is.
    //
    // Marked although ConnectorEnd is not, as SuccessionAsUsage is: this production's own
    // body is read in full, and ConnectorEnd's missing OwnedCrossMultiplicityMember is held
    // by tests/rejection/connector-end-cross-multiplicity-is-not-implemented.sysml.
    //
    // implied specialization: Links::selfLinks
    // constraint: BindingConnector::checkBindingConnectorSpecialization,
    //     `specializesFromLibrary('Links::selfLinks')` (KerML 8.3.4.5.2), which "requires
    //     that BindingConnectorAsUsages specialize the kernel Feature Links:selfLink"
    //     (SysML 8.4.9.3, receipt 6b0fb7c5). An injection, so sv2-hir's; this layer builds
    //     the tree only (ADR-0002).
    // constraint: BindingConnector::validateBindingConnectorIsBinary,
    //     `relatedFeature->size() = 2` (KerML 8.3.4.5.2). Holds by construction here: the
    //     production writes exactly two ends.
    fn binding_connector_as_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnectorAsUsage);
        self.usage_prefix();
        if self.at_keyword("binding") {
            self.bump_as(keyword("binding").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        }
        self.expect_keyword("bind");
        self.connector_end_member();
        self.expect(SyntaxKind::Eq, "`=`");
        self.connector_end_member();
        self.usage_body();
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
    // This production buys NO corpus file on its own, and that was measured before it
    // was written: all thirteen .sysml files that write `calc def` also write `return`,
    // and ReturnParameterMember (8.2.2.19) is unimplemented. It is here because the
    // keyword is a prerequisite for that member having a caller, not because a
    // first-error histogram put `calc` near the top — see the lesson recorded under the
    // action layer in .claude/state/state.json.
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
        self.start_node(SyntaxKind::CalculationBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.calculation_body_part();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a calculation or constraint declaration");
        }
        self.finish_node();
    }

    // CalculationBodyPart : Type =
    //     CalculationBodyItem* ( ownedRelationship += ResultExpressionMember )?
    //                                                            (SysML 8.2.2.19)
    //
    // NOT marked for coverage, and neither is CalculationBodyItem. The item is
    //
    //     CalculationBodyItem = ActionBodyItem | ReturnParameterMember
    //
    // and ReturnParameterMember is unimplemented, as are three of ActionBodyItem's four
    // alternatives — the initial nodes, successions and guards that are the action layer.
    // What IS reached is ActionBodyItem's first alternative, NonBehaviorBodyItem
    // (8.2.2.17.1), whose Import, AliasMember and DefinitionMember are the same three a
    // definition body reads. So `calc def C { return x; }` is reported and
    // `constraint def C { doc /* why */ a <= b }` is read, which is the shape the corpus
    // writes constraints in.
    //
    // The star is greedy and the expression is last; `at_result_expression` is where
    // that boundary is decided, and it is the whole of the difficulty here.
    fn calculation_body_part(&mut self) {
        self.eat_trivia();
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

    // RequirementConstraintUsage : ConstraintUsage =
    //     ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     RequirementBody
    //   | ( UsageExtensionKeyword* 'constraint' | UsageExtensionKeyword+ )
    //     ConstraintUsageDeclaration CalculationBody              (SysML 8.2.2.21.1)
    //
    // NOT marked for coverage. The second alternative's `UsageExtensionKeyword+` — prefix
    // metadata standing in for the `constraint` keyword entirely — is unimplemented, as
    // prefix metadata is everywhere in this parser. The `*` form with zero of them is
    // read, which is every instance the corpus writes.
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
    // conflict harmless to read: the second opens on the keyword `constraint` and the
    // first on a QualifiedName, and a keyword is not a name (SysML 8.2.2.1.2).
    fn requirement_constraint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RequirementConstraintUsage);
        if self.at_keyword("constraint") {
            self.bump_as(keyword("constraint").unwrap_or(SyntaxKind::BasicName));
            self.constraint_usage_declaration();
            self.calculation_body();
        } else {
            self.owned_reference_subsetting();
            if self.at_feature_specialization() {
                self.feature_specialization_part();
            }
            self.requirement_body();
        }
        self.finish_node();
    }

    // production: ConstraintUsageDeclaration
    //
    // ConstraintUsageDeclaration : ConstraintUsage =
    //     UsageDeclaration ValuePart?                            (SysML 8.2.2.20)
    //
    // Shared by three productions: RequirementConstraintUsage behind `require` or
    // `assume`, AssertConstraintUsage behind `assert constraint`, and ConstraintUsage, the
    // bare `constraint c { }` at member position.
    fn constraint_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstraintUsageDeclaration);
        self.usage_declaration();
        if self.at_value_part() {
            self.value_part();
        }
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
    /// `satisfy` comes after the same optional `not`: that is declined here, so it is
    /// reported rather than read as an assertion referencing `satisfy`, which is reserved
    /// and cannot be a name anyway.
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
            if self.at_feature_specialization() {
                self.feature_specialization_part();
            }
        }
        self.calculation_body();
        self.finish_node();
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

    // SubjectUsage : ReferenceUsage =
    //     'subject' UsageExtensionKeyword* Usage                (SysML 8.2.2.21.1)
    //
    // NOT marked for coverage. UsageExtensionKeyword is a PrefixMetadataMember
    // (8.2.2.6.2) and prefix metadata is unimplemented everywhere in this parser, so a
    // `subject #approved s;` is reported rather than read — the same gap UsagePrefix and
    // DefinitionPrefix carry, and recorded here for the same reason. The `*` makes zero
    // of them the common case, which is why the production is useful unmarked.
    //
    // The metaclass is ReferenceUsage, not a SubjectUsage of its own: what makes the
    // usage a subject is the membership that owns it, not the usage.
    fn subject_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SubjectUsage);
        self.expect_keyword("subject");
        self.usage();
        self.finish_node();
    }

    // production: PartDefinition
    // production: AttributeDefinition
    // production: OccurrenceDefinition
    // production: ItemDefinition
    // production: ConnectionDefinition
    // production: FlowDefinition
    // production: AllocationDefinition
    // production: RenderingDefinition
    //
    // AttributeDefinition  = DefinitionPrefix           'attribute'  'def' Definition
    // OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' Definition
    // ItemDefinition       = OccurrenceDefinitionPrefix 'item'       'def' Definition
    // PartDefinition       = OccurrenceDefinitionPrefix 'part'       'def' Definition
    // ConnectionDefinition = OccurrenceDefinitionPrefix 'connection' 'def' Definition
    // FlowDefinition       = OccurrenceDefinitionPrefix 'flow'       'def' Definition
    // AllocationDefinition = OccurrenceDefinitionPrefix 'allocation' 'def' Definition
    // RenderingDefinition  = OccurrenceDefinitionPrefix 'rendering'  'def' Definition
    //                                          (SysML 8.2.2.7, .9.1, .10, .11, .13,
    //                                                 .15, .16, .26.3)
    //
    // Eight productions, one method, as the seven usages share `simple_usage` and the
    // eight classifiers share `classifier`. Each is marked because each IS fully
    // implemented; what none of them implements lives in the prefixes and in
    // DefinitionDeclaration, and is recorded there.
    //
    // The Pilot factors 'part' 'def' into PartDefKeyword; deviations.json records
    // that as xtext_only/follow_spec, so the literals are matched here directly.
    fn simple_definition(&mut self, definition: SimpleDefinition) {
        self.eat_trivia();
        self.start_node(definition.node);
        if definition.is_occurrence {
            self.occurrence_definition_prefix();
        } else {
            self.definition_prefix();
        }
        self.expect_keyword(definition.keyword);
        self.expect_keyword("def");
        self.definition();
        self.finish_node();
    }

    // DefinitionPrefix : Definition =
    //     BasicDefinitionPrefix? DefinitionExtensionKeyword*      (SysML 8.2.2.6.1)
    //
    // NOT marked for coverage, for the reason OccurrenceDefinitionPrefix is not:
    // DefinitionExtensionKeyword (`#` prefix metadata) is unimplemented, and
    // `at_simple_definition` does not look past a `#`, so a definition carrying one
    // never reaches here and is reported by the enclosing body instead.
    //
    // This is OccurrenceDefinitionPrefix without the `individual` part. The two are
    // separate productions because only an occurrence may be individual, and keeping
    // them separate is what makes `individual attribute def A;` an error.
    fn definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionPrefix);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
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
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Definition);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a definition declaration");
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

    // production: RelationshipBody@sysml
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
            self.error_expected("`;` or `{` after an import declaration");
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
            p.annotating_element();
            p.finish_node();
        });
    }

    /// The `AnnotatingElement` alternation, shared by every place one may appear.
    ///
    /// `AnnotatingElement = Comment | Documentation | TextualRepresentation |
    /// MetadataUsage` in `SysML` 8.2.2.4.1, and the same with `MetadataFeature` in
    /// `KerML` 8.2.3.3.1 — the one difference is the fourth alternative, and neither
    /// spelling of it is implemented.
    ///
    /// An annotating element is reached three ways, and this is the one dispatch for all
    /// of them: `OwnedAnnotation` in a relationship body, `MemberElement` in `KerML`
    /// (8.2.3.4.1), and `DefinitionElement` in `SysML` (8.2.2.6.1). Writing the
    /// alternation twice is how the three would drift apart.
    ///
    /// The caller enters only on `at_annotating_element` or `at_annotating_member`, so
    /// the `else` never sees a metadata element.
    ///
    /// The caller is also responsible for `with_significant_comments`: every one of
    /// these productions ends in a `REGULAR_COMMENT` body, which is trivia unless the
    /// enclosing context has made it a token.
    fn annotating_element(&mut self) {
        if self.at_keyword("doc") {
            self.documentation();
        } else if self.at_keyword("rep") || self.at_keyword("language") {
            self.textual_representation();
        } else {
            self.comment();
        }
    }

    /// Whether a keyword-introduced `AnnotatingElement` starts at the `n`th token.
    ///
    /// Not the same question as `at_annotating_element`, deliberately. That one also
    /// answers yes to a bare `REGULAR_COMMENT`, which is `Comment`'s shortest form and
    /// is correct inside a relationship body, where every regular comment is either an
    /// annotation's body or an error.
    ///
    /// At member position it is not correct yet. A bare `/* ... */` in a package body IS
    /// a `Comment` element by 8.2.2.4.2, and this parser still attaches it as trivia —
    /// both readings keep every byte, so the round trip holds either way, but the tree
    /// shape differs from the specification's. Making the switch changes every tree that
    /// has a comment in it, so it is its own change with its own snapshot review rather
    /// than a side effect of this one.
    fn at_annotating_member(&self, n: usize) -> bool {
        ["comment", "locale", "doc", "rep", "language"]
            .iter()
            .any(|word| self.nth_is_keyword(n, word))
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

    // production: Classifier
    // production: Class
    // production: Structure
    // production: DataType
    // production: Metaclass
    // production: Association
    // production: Behavior
    // production: Interaction
    //
    // Classifier  = TypePrefix 'classifier'  ClassifierDeclaration TypeBody
    // Class       = TypePrefix 'class'       ClassifierDeclaration TypeBody
    // Structure   = TypePrefix 'struct'      ClassifierDeclaration TypeBody
    // DataType    = TypePrefix 'datatype'    ClassifierDeclaration TypeBody
    // Metaclass   = TypePrefix 'metaclass'   ClassifierDeclaration TypeBody
    // Association = TypePrefix 'assoc'       ClassifierDeclaration TypeBody
    // Behavior    = TypePrefix 'behavior'    ClassifierDeclaration TypeBody
    // Interaction = TypePrefix 'interaction' ClassifierDeclaration TypeBody
    //                                                            (KerML 8.2.4.2)
    //
    // Eight productions, one method, as the seven usages share `simple_usage`. Each is
    // marked separately because each IS fully implemented: what none of them implements
    // lives below, in ClassifierDeclaration's optional parts and in TypePrefix, and is
    // recorded there.
    fn classifier(&mut self, classifier: Classifier) {
        self.eat_trivia();
        self.start_node(classifier.node);
        self.type_prefix();
        self.expect_keyword(classifier.keyword);
        self.classifier_declaration();
        self.type_body();
        self.finish_node();
    }

    /// Which of `CLASSIFIERS` starts at the `n`th meaningful token, if any.
    fn at_classifier(&self, n: usize) -> Option<Classifier> {
        let after = self.skip_type_prefix(n);
        CLASSIFIERS
            .iter()
            .copied()
            .find(|c| self.nth_is_keyword(after, c.keyword))
    }

    /// The index just past a `TypePrefix` written from the `n`th token.
    fn skip_type_prefix(&self, n: usize) -> usize {
        n + usize::from(self.nth_is_keyword(n, "abstract"))
    }

    // TypePrefix : Type = ( isAbstract ?= 'abstract' )?
    //     ( ownedRelationship += PrefixMetadataMember )*          (KerML 8.2.4.1)
    //
    // NOT marked for coverage. PrefixMetadataMember — `#` prefix metadata — is
    // unimplemented, exactly as it is on OccurrenceDefinitionPrefix, and
    // `at_classifier` does not look past a `#`, so a classifier carrying one never
    // reaches here and is reported by the enclosing body instead.
    //
    // The node is built even when empty, as MemberPrefix's is.
    fn type_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypePrefix);
        self.eat_optional_keyword("abstract");
        self.finish_node();
    }

    // ClassifierDeclaration : Classifier =
    //     ( isSufficient ?= 'all' )? Identification
    //     ( ownedRelationship += OwnedMultiplicity )?
    //     ( SuperclassingPart | ConjugationPart )?
    //     TypeRelationshipPart*                                   (KerML 8.2.4.2)
    //
    // NOT marked for coverage. Three of its five parts are unimplemented, and each is a
    // construct the language has rather than an optional slot left empty:
    //
    //   - OwnedMultiplicity, the `[1..*]` on a classifier rather than on a feature.
    //   - ConjugationPart (`~` or `conjugates`), the second alternative of the one
    //     alternation here, held by tests/rejection/conjugation-part-is-not-implemented.kerml.
    //   - TypeRelationshipPart, the disjoining, unioning, intersecting and differencing
    //     parts, held by tests/rejection/type-relationship-part-is-not-implemented.kerml.
    //
    // `all` and Identification are read, and SuperclassingPart is fully implemented and
    // marked on its own below.
    fn classifier_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ClassifierDeclaration);
        self.eat_optional_keyword("all");
        self.identification();
        if self.at_superclassing() {
            self.superclassing_part();
        }
        self.finish_node();
    }

    /// Whether a `SuperclassingPart` is written here.
    ///
    /// `SPECIALIZES = ':>' | 'specializes'` (`KerML` 8.2.4.2). The symbol is checked
    /// before the word because the lexer gives the symbol its own kind while the word
    /// arrives as a `BasicName`.
    fn at_superclassing(&self) -> bool {
        self.at(SyntaxKind::ColonGt) || self.at_keyword("specializes")
    }

    // production: SuperclassingPart
    //
    // SuperclassingPart : Classifier =
    //     SPECIALIZES ownedRelationship += OwnedSubclassification
    //     ( ',' ownedRelationship += OwnedSubclassification )*    (KerML 8.2.4.2)
    //
    // OwnedSubclassification is a shared unit — the same production SysML's
    // SubclassificationPart owns — so the target is read by the method that already
    // exists for it rather than by a second one written here.
    fn superclassing_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuperclassingPart);
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

    // production: TypeBody
    //
    // TypeBody : Type = ';' | '{' TypeBodyElement* '}'            (KerML 8.2.4.1)
    //
    // TypeBodyElement is not marked: it is an alternation, read by `body_elements`, and
    // one of its four alternatives — FeatureMember — is unimplemented.
    fn type_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypeBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Type);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a classifier declaration");
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
    let (green, errors) = Parser::new(source, language).root_namespace();
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
