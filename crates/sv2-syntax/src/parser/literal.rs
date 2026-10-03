// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Literal expressions, `KerML` 8.2.5.8.4, and the null expression.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;

impl Parser<'_> {
    // production: NullExpression
    //
    // NullExpression : NullExpression = 'null' | '(' ')'         (KerML 8.2.5.8.3)
    pub(super) fn null_expression(&mut self) {
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

    /// Whether a `LiteralExpression` starts here (`KerML` 8.2.5.8.4).
    ///
    /// The five literals, by their opening token: `'true'`/`'false'`, a
    /// `STRING_VALUE`, a `DECIMAL_VALUE`, the `'.'` or `EXPONENTIAL_VALUE` a
    /// `RealValue` may open with, and the `'*'` of `LiteralInfinity`.
    pub(super) fn at_literal_expression(&self) -> bool {
        self.at(SyntaxKind::StringValue)
            || self.at(SyntaxKind::DecimalValue)
            || self.at(SyntaxKind::ExponentialValue)
            || self.at(SyntaxKind::Dot)
            || self.at(SyntaxKind::Star)
            || self.at_keyword("true")
            || self.at_keyword("false")
    }

    // production: LiteralExpression
    //
    // LiteralExpression = LiteralBoolean | LiteralString | LiteralInteger
    //                   | LiteralReal | LiteralInfinity          (KerML 8.2.5.8.4)
    //
    // All five alternatives, each marked below. An alternation with no node: the
    // literal says which.
    pub(super) fn literal_expression(&mut self) {
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
    // production: BooleanValue
    //
    // LiteralBoolean : LiteralBoolean = value = BooleanValue     (KerML 8.2.5.8.4)
    // BooleanValue = 'true' | 'false'                            (KerML 8.2.5.8.4)
    //
    // BooleanValue is a value production, not an element: its keyword is the
    // LiteralBoolean's only token, and it builds no node of its own.
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
    // production: RealValue
    //
    // LiteralReal : LiteralReal = value = RealValue              (KerML 8.2.5.8.4)
    //
    // RealValue = DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE )
    //           | EXPONENTIAL_VALUE                              (KerML 8.2.5.8.4)
    //
    // RealValue is a production and not a terminal, so a real is up to three tokens,
    // and the LiteralReal node is what holds them together: RealValue, a value
    // production like BooleanValue, builds none of its own. The leading DECIMAL_VALUE is
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
}
