// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Calculations and constraints, `SysML` 8.2.2.18 and 8.2.2.19: calculation bodies,
//! result expressions, and assert constraints.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;

impl Parser<'_> {
    /// Whether a `ConstraintDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'constraint' 'def'` (`SysML` 8.2.2.20). Only the
    /// `def` separates it from a `ConstraintUsage`.
    pub(super) fn at_constraint_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "constraint") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `CalculationDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'calc' 'def'` (`SysML` 8.2.2.19). Only the `def`
    /// separates it from a `CalculationUsage`.
    pub(super) fn at_calculation_definition(&self, n: usize) -> bool {
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
    pub(super) fn at_result_expression(&self) -> bool {
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

    /// Whether a `CalculationUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'calc'` with no `def` after it (`SysML` 8.2.2.19): the `def`
    /// is what makes it the `CalculationDefinition` beside it, as for `at_action_usage`.
    /// The prefix skipped is the one `calculation_usage` reads with
    /// `occurrence_usage_prefix`.
    pub(super) fn at_calculation_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "calc") && !self.nth_is_keyword(after + 1, "def")
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
    pub(super) fn calculation_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CalculationUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("calc");
        self.action_usage_declaration();
        self.calculation_body();
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
    pub(super) fn constraint_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConstraintDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("constraint");
        self.expect_keyword("def");
        self.definition_declaration();
        self.calculation_body();
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
    pub(super) fn calculation_definition(&mut self) {
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
    pub(super) fn calculation_body(&mut self) {
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
    pub(super) fn braced_calculation_body(&mut self) {
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
    pub(super) fn result_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ResultExpressionMember);
        self.member_prefix();
        self.owned_expression();
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
    pub(super) fn constraint_usage_declaration(&mut self) {
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
    pub(super) fn at_constraint_usage(&self, n: usize) -> bool {
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
    pub(super) fn constraint_usage(&mut self) {
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
    pub(super) fn at_assert_constraint_usage(&self, n: usize) -> bool {
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
    pub(super) fn assert_constraint_usage(&mut self) {
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
}
