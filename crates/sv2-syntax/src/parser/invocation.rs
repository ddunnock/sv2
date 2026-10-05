// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Invocation and constructor expressions and their argument lists, `KerML` 8.2.5.8.2.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::operator::TIER_LOOSEST;

impl Parser<'_> {
    /// Whether a `'('` here is immediately closed, making it a `NullExpression`.
    pub(super) fn at_empty_parentheses(&self) -> bool {
        self.at(SyntaxKind::LParen) && self.nth_is(1, SyntaxKind::RParen)
    }

    /// Whether an `InvocationExpression` starts here.
    ///
    /// A `QualifiedName`, or a chain of them, with a `'('` after it — the whole of what
    /// separates it from a `FeatureReferenceExpression`, which is the same name with
    /// nothing after it, and from a `FeatureChainExpression`, the chain with nothing after
    /// it (`KerML` 8.2.5.8.2-3).
    ///
    /// It asks for a NAME rather than for any token before the `(`, which is what keeps
    /// `x and (y)` an operator over a parenthesised operand: `and` is reserved
    /// (`SysML` 8.2.2.1.2) and a keyword is not a name, so `nth_is_name` says no. A `(`
    /// with nothing before it never reaches here at all — `null_expression` and
    /// `sequence_expression` are asked first.
    pub(super) fn at_invocation_expression(&self) -> bool {
        self.skip_instantiated_type_member(0)
            .is_some_and(|n| self.nth_is(n, SyntaxKind::LParen))
    }

    /// The index just past an `InstantiatedTypeMember` written at the `n`th meaningful
    /// token: a `QualifiedName`, and any `'.'`-joined names after it, as
    /// `instantiated_type_member` reads them.
    ///
    /// One pass, as `skip_qualified_name` is: it walks a whole `a.b.c...` chain at every
    /// base expression, which was quadratic by index while `peek_nth` filtered.
    fn skip_instantiated_type_member(&self, n: usize) -> Option<usize> {
        let mut tokens = self.meaningful_from(n);
        let mut length = self.qualified_name_length(&mut tokens)?;
        loop {
            let mut ahead = tokens.clone();
            let dot = ahead.next().is_some_and(|t| t.kind == SyntaxKind::Dot);
            if !(dot && ahead.next().is_some_and(|t| self.is_name(t))) {
                return Some(n + length);
            }
            tokens.next();
            length += 1 + self.qualified_name_length(&mut tokens)?;
        }
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
    // The invoked type is an InstantiatedTypeMember, a name or a feature chain; see
    // `instantiated_type_member`.
    //
    // The EmptyResultMember is the result parameter every invocation owns and nobody
    // writes, exactly as FeatureReferenceExpression owns one (8.2.5.8.3 names it in both).
    pub(super) fn invocation_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InvocationExpression);
        self.instantiated_type_member();
        self.argument_list();
        self.empty_result_member();
        self.finish_node();
    }

    // production: InstantiatedTypeMember
    //
    // InstantiatedTypeMember : Membership =
    //       memberElement = InstantiatedTypeReference
    //     | OwnedFeatureChainMember                              (KerML 8.2.5.8.3)
    //
    // production: OwnedFeatureChainMember@kerml
    //
    // OwnedFeatureChainMember : OwningMembership =
    //     ownedMemberElement = FeatureChain                      (KerML 8.2.5.8.2)
    //
    // production: FeatureChain@kerml
    //
    // FeatureChain : Feature =
    //     ownedRelationship += OwnedFeatureChaining
    //     ( '.' ownedRelationship += OwnedFeatureChaining )+     (KerML 8.2.4.3.5)
    //
    // The invoked type of an InvocationExpression, a FunctionOperationExpression and a
    // ConstructorExpression: a name, or a chain of two or more, `a.b(x)`. A SHARED unit,
    // whose OwnedFeatureChainMember is each language's own: SysML's (8.2.2.17.5) owns an
    // OwnedFeatureChain, `ownedRelationship += OwnedFeatureChaining ( '.' ... )+`
    // (8.2.2.6.5), and KerML's owns a FeatureChain of the same text, so both are read by
    // `owned_feature_chain` and build the same OwnedFeatureChain node, whose element is a
    // Feature either way. NODE KIND, deliberately: KerML's FeatureChain builds an
    // `OwnedFeatureChain` node, not a `FeatureChain` one, which does not exist; KerML's own
    // `OwnedFeatureChain : Feature = FeatureChain` (8.2.4.3.5) is the same element, so a
    // typed accessor in sv2-ast reads this node for both. The Pilot states the one rule for both
    // (KerMLExpressions.xtext:436-438, `ownedRelatedElement += OwnedFeatureChain`).
    //
    // `a.b(x)` is otherwise `a`, a postfix `.b`, and a `(` nothing can read; the Pilot
    // takes the invocation by backtracking out of that, and here the chain is looked past
    // to the `(` instead (`at_invocation_expression`). A chain is two or more names, so a
    // lone name is the InstantiatedTypeReference and never a chain of one.
    //
    // Deviations InstantiatedTypeReference and OwnedFeatureChainMember are spec_only,
    // follow_spec: implement what the specification states. The corpus writes the chain
    // in SysML Annex A (SimpleVehicleModel.sysml:1232,
    // `vehicleSpecification.vehicleMassRequirement(vehicle_uut)`) and in KerML
    // (examples/Simple Tests/Expressions.kerml:56, `f.s(1)`).
    pub(super) fn instantiated_type_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InstantiatedTypeMember);
        if self.at_owned_feature_chain() {
            self.start_node(SyntaxKind::OwnedFeatureChainMember);
            let start = self.builder.checkpoint();
            self.qualified_name();
            self.owned_feature_chain(start);
            self.finish_node();
        } else {
            self.start_node(SyntaxKind::InstantiatedTypeReference);
            self.qualified_name();
            self.finish_node();
        }
        self.finish_node();
    }

    // production: ConstructorExpression
    //
    // ConstructorExpression =
    //     'new' ownedRelationship += InstantiatedTypeMember
    //     ownedRelationship += ConstructorResultMember               (KerML 8.2.5.8.3)
    //
    // production: ConstructorResultMember
    //
    // ConstructorResultMember : ReturnParameterMembership =
    //     ownedRelatedElement += ConstructorResult                   (KerML 8.2.5.8.3)
    //
    // production: ConstructorResult
    //
    // ConstructorResult : Feature = ArgumentList                    (KerML 8.2.5.8.3)
    //
    // "the keyword new followed by the qualified name of a type to be instantiated ...
    // followed by a parenthesized list of argument expressions, similarly to an invocation
    // expression" (KerML 7.4.9.4, receipt f77ceb64). A shared unit: SysML reaches it
    // through the same BaseExpression.
    //
    // `new` is printed as reserved in SysML only. The SysML Tier B' BNF lists it among the
    // reserved keywords (vendor/spec-bnf/SysML-textual-bnf.kebnf:20); KerML's list does not
    // (KerML-textual-bnf.kebnf RESERVED_KEYWORD, and KerML 8.2.2.6), although this very
    // production writes it as a literal. KerML reserves it anyway, by deviation
    // ConstructorExpression (follow_xtext): the pilot lexer reserves it in both languages
    // and the corpus never names anything `new`. So `feature new;` is rejected in a .kerml
    // file too, and tests/rejection/new-is-reserved-in-kerml.kerml holds that. It admits no
    // text, so there is no PARSE-DEVIATION site for it (.claude/state/deviation-sites-pending.txt).
    //
    // The ArgumentList is not the expression's own, as it is an InvocationExpression's: it
    // belongs to the ConstructorResult, the result parameter the arguments bind features
    // of — "binding some or all of the features of the instantiatedType to the results of
    // its argument Expressions" (8.3.4.8.3, receipt 554f13c6). So there is NO
    // EmptyResultMember here, and the clause names none; the ConstructorResultMember is the
    // result.
    //
    // InstantiatedTypeMember is read by `instantiated_type_member`, as the invocation's is.
    //
    // implied specialization: Performances::constructorEvaluations
    // constraint: ConstructorExpression::checkConstructorExpressionSpecialization
    //     `specializes('Performances::constructorEvaluations')` (8.3.4.8.3). Injections
    //     belong in sv2-hir; this layer builds the tree only (ADR-0002).
    pub(super) fn constructor_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstructorExpression);
        self.expect_keyword("new");
        self.instantiated_type_member();
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstructorResultMember);
        self.start_node(SyntaxKind::ConstructorResult);
        self.argument_list();
        self.finish_node();
        self.finish_node();
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
    pub(super) fn argument_list(&mut self) {
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
    pub(super) fn argument_member(&mut self, tier: u8) {
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
    pub(super) fn argument_expression_member(&mut self, tier: u8) {
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
    pub(super) fn empty_result_member(&mut self) {
        self.start_node(SyntaxKind::EmptyResultMember);
        self.start_node(SyntaxKind::EmptyFeature);
        self.finish_node();
        self.finish_node();
    }
}
