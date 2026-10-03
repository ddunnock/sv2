// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Actions, `SysML` 8.2.2.16: action definitions, usages and bodies, and perform
//! action usages.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;

impl Parser<'_> {
    /// Whether an `ActionDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'action' 'def'` (`SysML` 8.2.2.17.1). Only the `def`
    /// separates it from an `ActionUsage`.
    pub(super) fn at_action_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "action") && self.nth_is_keyword(after + 1, "def")
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
    pub(super) fn at_action_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "action") && !self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `PerformActionUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'perform'` (`SysML` 8.2.2.17.2). No `def` test, because
    /// there is no `perform def`: the keyword names a usage and nothing else.
    pub(super) fn at_perform_action_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "perform")
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
    pub(super) fn action_definition(&mut self) {
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
    // ActionBodyItem is marked at `body_element`, which reads every one of its
    // alternatives under Body::Action (`every_body_admits_exactly_its_item_production`
    // holds them):
    //
    //     ActionBodyItem = NonBehaviorBodyItem
    //                    | InitialNodeMember ActionTargetSuccessionMember*
    //                    | SourceSuccessionMember? ActionBehaviorMember
    //                      ActionTargetSuccessionMember*
    //                    | GuardedSuccessionMember
    pub(super) fn action_body(&mut self) {
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
    pub(super) fn action_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("action");
        self.action_usage_declaration();
        self.action_body();
        self.finish_node();
    }

    pub(super) fn action_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionUsageDeclaration);
        self.usage_declaration();
        if self.at_value_part() {
            self.value_part();
        }
        self.finish_node();
    }

    /// An `EmptyActionUsage`, built from no tokens.
    pub(super) fn empty_action_usage(&mut self) {
        self.start_node(SyntaxKind::EmptyActionUsage);
        self.finish_node();
    }

    // production: ActionBodyParameterMember@sysml
    //
    // ActionBodyParameterMember : ParameterMembership =
    //     ownedRelatedElement += ActionBodyParameter                 (SysML 8.2.2.17.7)
    //
    // production: ActionBodyParameter@sysml
    //
    // ActionBodyParameter : ActionUsage =
    //     ( 'action' UsageDeclaration? )?
    //     '{' ActionBodyItem* '}'                                    (SysML 8.2.2.17.7)
    //
    // "the body clause is itself notated as an action usage, but with its body required
    // to be given using curly braces { … }, with a semicolon not allowed for an empty
    // body" (7.17.12, receipt b0446148): the braces are the production's own, not an
    // ActionBody's, which would admit `;`. The items are an ActionBody's, read by the same
    // loop.
    pub(super) fn action_body_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ActionBodyParameterMember);
        self.start_node(SyntaxKind::ActionBodyParameter);
        if self.at_keyword("action") {
            self.expect_keyword("action");
            if !self.at(SyntaxKind::LBrace) {
                self.usage_declaration();
            }
        }
        if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Action);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`{` opening a loop or branch body");
        }
        self.finish_node();
        self.finish_node();
    }

    /// `( '{' ActionBodyItem* '}' )?`, an effect's body: braces or nothing.
    pub(super) fn optional_action_body_items(&mut self) {
        if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Action);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        }
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
    pub(super) fn perform_action_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PerformActionUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("perform");
        self.perform_action_usage_declaration();
        self.action_body();
        self.finish_node();
    }

    pub(super) fn perform_action_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PerformActionUsageDeclaration);
        if self.at_keyword("action") {
            self.bump_as(keyword("action").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.finish_node();
    }

    /// Whether a `ReturnParameterMember` starts here.
    ///
    /// `MemberPrefix? 'return'` (`SysML` 8.2.2.19). `return` is a pinned keyword and a
    /// keyword is not a name (`SysML` 8.2.2.1.2), so nothing else can open on it: it
    /// cannot be the bare name a `DefaultReferenceUsage` opens on, and it cannot be the
    /// first token of the trailing `ResultExpressionMember` either.
    pub(super) fn at_return_parameter_member(&self) -> bool {
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
    // UsageElement (8.2.2.5.2) is read by `usage_element`, which returns whether it read
    // one, and is marked at `usage_element_of_class`.
    pub(super) fn return_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ReturnParameterMember);
        self.member_prefix();
        self.expect_keyword("return");
        if !self.usage_element() {
            self.error_expected("a usage after `return`");
        }
        self.finish_node();
    }
}
