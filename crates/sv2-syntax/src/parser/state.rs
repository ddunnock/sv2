// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! States and transitions, `SysML` 8.2.2.17: state bodies, entry, do and exit actions,
//! triggers, guards and effects.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;
use crate::parser::operator::TIER_LOOSEST;

impl Parser<'_> {
    /// Whether a `StateDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'state' 'def'` (`SysML` 8.2.2.18.1): the `def` is what
    /// separates it from a `StateUsage`, as for `at_action_definition`.
    pub(super) fn at_state_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "state") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether a `StateUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'state'` with no `def` after it (`SysML` 8.2.2.18.2).
    /// `exhibit state` is an `ExhibitStateUsage`, and `exhibit` is not looked past here:
    /// `at_exhibit_state_usage` answers for it.
    pub(super) fn at_state_usage(&self, n: usize) -> bool {
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
    pub(super) fn state_definition(&mut self) {
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
    pub(super) fn state_usage(&mut self) {
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
    pub(super) fn transition_usage_member(&mut self) {
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
    pub(super) fn at_target_transition_usage_member(&self) -> bool {
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
    pub(super) fn target_transition_usage_member(&mut self) {
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
    pub(super) fn empty_parameter_member(&mut self) {
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
    pub(super) fn accept_parameter_part(&mut self) {
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
    pub(super) fn state_action_item(&mut self) {
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
    pub(super) fn at_exhibit_state_usage(&self, n: usize) -> bool {
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
    pub(super) fn exhibit_state_usage(&mut self) {
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
    pub(super) fn transition_succession_member(&mut self) {
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
}
