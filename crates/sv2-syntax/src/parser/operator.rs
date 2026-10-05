// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The operator table of `KerML` 8.2.5.8.1, table 6, read from
//! `docs/operator-precedence.toml`, and the helpers that consume operator spellings.

use crate::generated::kinds::{OPERATORS, SyntaxKind};
use crate::parser::Parser;
use crate::parser::lookahead::keyword;

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
pub(super) enum Assoc {
    /// `a - b - c` is `(a - b) - c`. Every binary tier but exponentiation.
    Left,
    /// `a ** b ** c` is `a ** (b ** c)`. Exponentiation alone, by note 2.
    Right,
}

/// How an operator is written: a symbol the lexer gives its own kind, or a word that
/// arrives as a `BasicName` and is separated from a name by the pinned keyword table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Spelling {
    Symbol(SyntaxKind),
    Word(&'static str),
}

/// The membership an operand is owned through, which is not the same for every operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Operand {
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
pub(super) struct InfixOperator {
    pub(super) spelling: Spelling,
    /// The tier from table 6. Lower binds tighter.
    pub(super) tier: u8,
    pub(super) assoc: Assoc,
    /// The node this operator builds.
    pub(super) node: SyntaxKind,
    /// The memberships the left operand is wrapped in, outermost first.
    pub(super) left: &'static [SyntaxKind],
    /// The membership the right operand is read into.
    pub(super) right: Operand,
}

/// The membership chain an `ArgumentMember` left operand is wrapped in.
pub(super) const LEFT_ARGUMENT: &[SyntaxKind] = &[
    SyntaxKind::ArgumentMember,
    SyntaxKind::Argument,
    SyntaxKind::ArgumentValue,
];

/// Tier 2, `UnaryOperatorExpression`. TIGHTER than exponentiation at tier 3, so
/// `-2 ** 2` is `(-2) ** 2` — the opposite of the C and Python convention.
pub(super) const TIER_UNARY: u8 = 2;

/// Tier 8, the classification and metaclassification operators.
const TIER_CLASSIFICATION: u8 = 8;

/// Tier 15, `ConditionalExpression`. The loosest tier in table 6.
pub(super) const TIER_CONDITIONAL: u8 = 15;

/// The loosest tier an expression may open at, which is every tier.
pub(super) const TIER_LOOSEST: u8 = TIER_CONDITIONAL;

/// `UnaryOperator = '+' | '-' | '~' | 'not'` (`KerML` 8.2.5.8.1), tier 2.
pub(super) const UNARY_OPERATORS: [Spelling; 4] = [
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
pub(super) const INFIX: [InfixOperator; 27] = [
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

impl Parser<'_> {
    // -- reading the precedence table --------------------------------------------

    /// The infix operator written here, or `None` if the next token is not one.
    ///
    /// Symbols are checked before words, as `at_feature_specialization` does: the
    /// lexer gives each symbol its own kind while every word arrives as a
    /// `BasicName`, so a symbol is one comparison and a word is a text match.
    pub(super) fn infix_operator_here(&self) -> Option<&'static InfixOperator> {
        INFIX.iter().find(|op| match op.spelling {
            Spelling::Symbol(kind) => self.at(kind),
            Spelling::Word(word) => self.at_keyword(word),
        })
    }

    /// Whether a `UnaryOperator` is written here (`KerML` 8.2.5.8.1).
    pub(super) fn at_unary_operator(&self) -> bool {
        UNARY_OPERATORS.iter().any(|spelling| match spelling {
            Spelling::Symbol(kind) => self.at(*kind),
            Spelling::Word(word) => self.at_keyword(word),
        })
    }

    /// Whether a `ClassificationExpression` opens here with no left operand.
    ///
    /// Its `ArgumentMember` is the one optional operand in the clause, so a
    /// classification or cast operator may be the first token of an expression.
    pub(super) fn at_leading_classification(&self) -> bool {
        self.at(SyntaxKind::At) || self.nth_is_any_keyword(0, &["istype", "hastype", "as"])
    }

    /// Whether a `MetaclassificationExpression` starts here.
    ///
    /// Decided by looking past the `QualifiedName` that is its left operand: `x meta
    /// T` is one and `x` alone is a `FeatureReferenceExpression`, and the two differ
    /// only after the name. `at_metaclassification` is asked before the name is
    /// parsed, because a `MetadataArgumentMember` cannot be wrapped around a
    /// `FeatureReferenceExpression` that is already in the tree.
    pub(super) fn at_metaclassification(&self) -> bool {
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

    // production: ClassificationTestOperator
    // production: CastOperator
    //
    // ClassificationTestOperator = 'istype' | 'hastype' | '@'    (KerML 8.2.5.8.1)
    // CastOperator              = 'as'                           (KerML 8.2.5.8.1)
    //
    // Value productions, no node. Every spelling of both is read in two places: here,
    // when a ClassificationExpression opens with no left operand (`@T`, `istype T`,
    // `as T`), and as a row of INFIX, bumped by `infix_tail`, when it has one. The
    // marker sits here, where the productions are spelled out whole.
    //
    /// Consume the operator token of a `ClassificationExpression`.
    pub(super) fn bump_classification_operator(&mut self) {
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

    // production: UnaryOperator
    //
    // UnaryOperator = '+' | '-' | '~' | 'not'                    (KerML 8.2.5.8.1)
    //
    // A value production, every spelling a row of UNARY_OPERATORS; no node.
    //
    /// Consume the operator token of a `UnaryOperatorExpression`.
    pub(super) fn bump_unary_operator(&mut self) {
        match UNARY_OPERATORS.iter().find(|spelling| match spelling {
            Spelling::Symbol(kind) => self.at(*kind),
            Spelling::Word(word) => self.at_keyword(word),
        }) {
            Some(spelling) => self.bump_spelling(*spelling),
            None => self.error_expected("`+`, `-`, `~` or `not`"),
        }
    }

    /// Consume the token an operator is spelled with, tagged as the token set names it.
    pub(super) fn bump_spelling(&mut self, spelling: Spelling) {
        match spelling {
            Spelling::Symbol(_) => self.bump(),
            Spelling::Word(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
        }
    }
}
