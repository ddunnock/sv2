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
}

/// Every usage production that is a prefix, one keyword and the `Usage` spine.
///
/// Ordered as the clauses number them. The keywords are disjoint, so the order does
/// not decide anything — `at_simple_usage` takes the one whose keyword is written.
const SIMPLE_USAGES: [SimpleUsage; 7] = [
    SimpleUsage {
        keyword: "attribute",
        node: SyntaxKind::AttributeUsage,
        is_occurrence: false,
    },
    SimpleUsage {
        keyword: "enum",
        node: SyntaxKind::EnumerationUsage,
        is_occurrence: false,
    },
    SimpleUsage {
        keyword: "occurrence",
        node: SyntaxKind::OccurrenceUsage,
        is_occurrence: true,
    },
    SimpleUsage {
        keyword: "item",
        node: SyntaxKind::ItemUsage,
        is_occurrence: true,
    },
    SimpleUsage {
        keyword: "part",
        node: SyntaxKind::PartUsage,
        is_occurrence: true,
    },
    SimpleUsage {
        keyword: "port",
        node: SyntaxKind::PortUsage,
        is_occurrence: true,
    },
    SimpleUsage {
        keyword: "rendering",
        node: SyntaxKind::RenderingUsage,
        is_occurrence: true,
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
    /// The braced form of `TypeBody`. `KerML` only — what a classifier holds.
    Type,
}

impl Body {
    /// The membership node a nested element is owned through.
    fn member(self, language: Language) -> SyntaxKind {
        match (self, language) {
            (Self::Definition, _) => SyntaxKind::DefinitionMember,
            (_, Language::SysMl) => SyntaxKind::PackageMember,
            // Both KerML bodies own the same membership. `TypeBodyElement` is
            // `NonFeatureMember | FeatureMember | AliasMember | Import` (8.2.4.1) and
            // `NamespaceBodyElement` reaches `NonFeatureMember` too (8.2.3.4.1), so
            // `Body::Type` needs no arm of its own. `FeatureMember` is unimplemented in
            // both, which is what leaves them identical for now rather than by rule.
            (_, Language::KerMl) => SyntaxKind::NonFeatureMember,
        }
    }

    /// Whether `ElementFilterMember` is one of this body's alternatives.
    fn admits_filter(self, language: Language) -> bool {
        match self {
            Self::Package => true,
            Self::Root => language == Language::SysMl,
            // TypeBodyElement has no ElementFilterMember alternative.
            Self::Definition | Self::Type => false,
        }
    }
}

/// A `SysML` definition production: one keyword over a shared spine.
///
/// Eight productions of `SysML` 8.2.2 are stated as `<prefix> KEYWORD 'def' Definition`,
/// differing in the keyword and in which prefix they take. Matching rule shapes across
/// the verified units finds twenty-two productions with a `def` keyword; the other
/// fourteen end in a specialised body — `ActionBody`, `CaseBody`, `CalculationBody`,
/// `RequirementBody`, `StateDefBody`, `InterfaceBody`, `ViewDefinitionBody` — and not
/// one of those bodies is implemented, so not one of them is here. `PortDefinition`
/// shares the spine but carries a trailing `ConjugatedPortDefinitionMember`, which is
/// also unimplemented, so it is out too.
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
    /// `skip_basic_usage_prefix` and `skip_usage_prefix` make one level down.
    fn skip_occurrence_definition_prefix(&self, n: usize) -> usize {
        let n = self.skip_definition_prefix(n);
        n + usize::from(self.nth_is_keyword(n, "individual"))
    }

    /// Whether an implemented `DefinitionElement` starts at the `n`th meaningful token.
    fn at_definition_element(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "package") || self.at_simple_definition(n).is_some()
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
    /// `Package` is the one `NonFeatureElement` implemented, and it is a shared unit —
    /// the same production in both grammars. Nothing else is: `FeatureElement`'s ten
    /// alternatives (`feature`, `connector`, `succession`, `flow` and the rest) and the
    /// remaining `NonFeatureElement`s (`class`, `struct`, `assoc`, `behavior`, …) are
    /// unimplemented, so they are reported rather than read.
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
            Language::KerMl => self.nth_is_keyword(n, "package") || self.at_classifier(n).is_some(),
            Language::SysMl => self.at_definition_element(n) || self.at_simple_usage(n).is_some(),
        }
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
    fn skip_usage_prefix(&self, n: usize) -> usize {
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
    fn emit(&mut self, code: DiagnosticCode, range: TextRange, message: String) {
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

    // production: RootNamespace
    //
    // RootNamespace = PackageBodyElement*                          (SysML 8.2.2.5.1)
    // RootNamespace = NamespaceBodyElement*                        (KerML 8.2.3.4.1)
    //
    // The one production the two grammars state differently at the start symbol, which
    // is what ADR-0014 exists for. `Body::Root` carries the difference; the loop below
    // is shared, because everything else about reading a body is.
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
            } else if self.at_member_element(usize::from(self.at_visibility())) {
                self.membership(body);
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
    // the only two of `DefinitionElement`'s 30 alternatives implemented, and the
    // seven in SIMPLE_USAGES are what is implemented of `UsageElement`'s.
    //
    // A definition is tried before a usage. The two share every prefix keyword and
    // the keyword after them, so `at_part_definition` — which requires the `def` —
    // must decide first; the usages are what is left.
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
    fn membership(&mut self, body: Body) {
        self.eat_trivia();
        self.start_node(body.member(self.language));
        self.member_prefix();
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
        } else if let Some(definition) = self
            .at_simple_definition(0)
            .filter(|_| self.language == Language::SysMl)
        {
            self.simple_definition(definition);
        } else if let Some(usage) = self.at_simple_usage(0).filter(|_| {
            // A usage is a UsageElement, reachable from PackageMember and not from
            // NamespaceMember. KerML has no usages at all (SysML 8.2.2.6.1).
            self.language == Language::SysMl
        }) {
            self.simple_usage(usage);
        } else if self.language == Language::KerMl {
            self.error_expected("a package or a classifier");
        } else {
            self.error_expected("a package, a part definition or a usage");
        }
        self.finish_node();
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
    fn at_simple_usage(&self, n: usize) -> Option<SimpleUsage> {
        SIMPLE_USAGES.iter().copied().find(|usage| {
            let after = if usage.is_occurrence {
                self.skip_usage_prefix(n)
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
        const SYMBOLS: [SyntaxKind; 5] = [
            SyntaxKind::Colon,        // DEFINED_BY
            SyntaxKind::ColonGt,      // SUBSETS
            SyntaxKind::ColonGtGt,    // REDEFINES
            SyntaxKind::ColonColonGt, // REFERENCES
            SyntaxKind::FatArrow,     // CROSSES
        ];
        SYMBOLS.iter().any(|kind| self.at(*kind))
            || ["subsets", "redefines", "references", "crosses"]
                .iter()
                .any(|word| self.at_keyword(word))
            || (self.at_keyword("defined") && self.nth_is_keyword(1, "by"))
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

    // production: OwnedSubsetting
    //
    // OwnedSubsetting : Subsetting =
    //     subsettedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // The OwnedFeatureChain alternative is not implemented; see owned_feature_typing.
    fn owned_subsetting(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedSubsetting);
        self.qualified_name();
        self.finish_node();
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

    // production: OwnedRedefinition
    //
    // OwnedRedefinition : Redefinition =
    //     redefinedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    fn owned_redefinition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedRedefinition);
        self.qualified_name();
        self.finish_node();
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

    // production: OwnedReferenceSubsetting
    //
    // OwnedReferenceSubsetting : ReferenceSubsetting =
    //     referencedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    fn owned_reference_subsetting(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedReferenceSubsetting);
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

    // production: OwnedCrossSubsetting
    //
    // OwnedCrossSubsetting : CrossSubsetting =
    //     crossedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    fn owned_cross_subsetting(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedCrossSubsetting);
        self.qualified_name();
        self.finish_node();
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

    // production: Typings
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

    // production: TypedBy
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
            self.expect_keyword("defined");
            self.expect_keyword("by");
        }
        self.feature_typing();
        self.finish_node();
    }

    // production: FeatureTyping
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

    // production: OwnedFeatureTyping
    //
    // OwnedFeatureTyping : FeatureTyping =
    //     type = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    //                                                            (SysML 8.2.2.6.5)
    //
    // OwnedFeatureChain needs two or more segments joined by '.', and a FeatureChain
    // has at least two by construction, so a bare QualifiedName is never ambiguous
    // with one. The chain alternative is not implemented.
    fn owned_feature_typing(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedFeatureTyping);
        self.qualified_name();
        self.finish_node();
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

    // production: OwnedMultiplicity
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

    // production: MultiplicityRange
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
        // NullExpression = 'null' | '(' ')' — the empty pair is decided before
        // SequenceExpression, which would otherwise read the '(' and find no
        // expression.
        if self.at_keyword("null") || self.at_empty_parentheses() {
            self.null_expression();
        } else if self.at(SyntaxKind::LParen) {
            self.sequence_expression();
        } else if self.at_literal_expression() {
            self.literal_expression();
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

    // production: PackageBody
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
