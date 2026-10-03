// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Requirements and concerns, `SysML` 8.2.2.20 and 8.2.2.21: subjects, actors,
//! stakeholders, framed concerns, satisfy and verify.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;

impl Parser<'_> {
    /// Whether a `RequirementDefinition` starts at the `n`th meaningful token.
    ///
    /// Its own question rather than a row in `SIMPLE_DEFINITIONS`, because it is not on
    /// that spine: it takes a `DefinitionDeclaration` and a `RequirementBody` directly,
    /// where the eight take a `Definition` — which is a declaration and a
    /// `DefinitionBody`. The prefix is the same one a part takes, so only `requirement`
    /// followed by `def` decides; `requirement r;` is a `RequirementUsage` and
    /// unimplemented.
    pub(super) fn at_requirement_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "requirement") && self.nth_is_keyword(after + 1, "def")
    }

    /// `RequirementBodyItem`'s six members of its own (`SysML` 8.2.2.21.1), each owning its
    /// element through a membership of its own. Returns whether one was read. Split out of
    /// `body_specific_item` for clippy's complexity budget; the arms are disjoint on their
    /// keywords, so where they are asked decides nothing.
    pub(super) fn requirement_body_member(&mut self, body: Body) -> bool {
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

    /// A `RequirementDefinition`, `ConcernDefinition`, `ViewpointDefinition`,
    /// `ConstraintDefinition` or `CalculationDefinition` read as a `DefinitionElement`,
    /// returning whether one was.
    /// Split out of `definition_element` so that function stays within clippy's complexity
    /// budget; each opens on its own keyword pair, so the order decides nothing.
    pub(super) fn requirement_family_definition(&mut self) -> bool {
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
    pub(super) fn at_concern_definition(&self, n: usize) -> bool {
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
    pub(super) fn requirement_body(&mut self) {
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

    /// Whether a `RequirementUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'requirement'` with no `def` after it (`SysML` 8.2.2.21.2):
    /// the `def` is what makes it the `RequirementDefinition` beside it. The prefix skipped
    /// is the one `requirement_usage` reads with `occurrence_usage_prefix`.
    pub(super) fn at_requirement_usage(&self, n: usize) -> bool {
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
    pub(super) fn requirement_usage(&mut self) {
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
    pub(super) fn at_concern_usage(&self, n: usize) -> bool {
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
    pub(super) fn concern_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConcernUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("concern");
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `SatisfyRequirementUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'assert'? 'not'? 'satisfy'` (`SysML` 8.2.2.21.2, with the two
    /// `?` of deviation `SatisfyRequirementUsage`). `satisfy` is reserved and opens nothing
    /// else, so it decides, and `at_assert_constraint_usage` declines exactly what this
    /// accepts after an `assert`.
    pub(super) fn at_satisfy_requirement_usage(&self, n: usize) -> bool {
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
    pub(super) fn satisfy_requirement_usage(&mut self) {
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
}
