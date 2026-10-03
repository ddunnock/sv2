// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Successions, `SysML` 8.2.2.16 and 8.2.2.15: initial nodes, target, guarded and
//! default successions, connector ends, and successions as usages.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::{VISIBILITY, keyword};
use crate::parser::usage::UsageClass;

impl Parser<'_> {
    /// A `SourceSuccessionMember` and the member it prefixes, as one item.
    ///
    /// `at_source_succession_member` has already seen an occurrence usage after the
    /// `then`, so `membership` builds the member the body's item production names for
    /// it: `OccurrenceUsageMember` in a definition body, `StructureUsageMember` or
    /// `BehaviorUsageMember` in an action body (8.2.2.6.1, 8.2.2.17.1).
    pub(super) fn source_succession_item(&mut self, body: Body) {
        self.source_succession_member();
        let element = self.membership(body);
        self.behaviour_targets(body, element);
    }

    /// Whether a `SuccessionFlowUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'succession' 'flow'` (`SysML` 8.2.2.16). Both keywords are
    /// reserved (8.2.2.1.2), so the pair decides alone: no `SuccessionAsUsage` writes
    /// `flow` after its `succession`, and there is no succession flow definition, so no
    /// `def` test.
    pub(super) fn at_succession_flow_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "succession") && self.nth_is_keyword(after + 1, "flow")
    }

    // production: SuccessionFlowUsage@sysml
    //
    // SuccessionFlowUsage =
    //     OccurrenceUsagePrefix 'succession' 'flow'
    //     FlowDeclaration DefinitionBody                         (SysML 8.2.2.16)
    //
    // "A flow usage is declared as a succession flow like a streaming flow above, but
    // using the keyword succession flow" (7.16.2, receipt 13d6f883): FlowUsage's shape
    // with two keywords for its one, so FlowDeclaration is read by the same method. The
    // metaclass is SuccessionFlowUsage (8.3.16.4, receipt 66280078), "a FlowUsage that is
    // also a KerML SuccessionFlow". The Pilot factors the keywords into
    // SuccessionFlowKeyword; deviation SuccessionFlowKeyword (xtext_only, follow_spec)
    // says to match the literals, so there is no production for it.
    //
    // A StructureUsageElement (8.2.2.6.4), as FlowUsage is.
    //
    // implied specialization: Flows::successionFlows ("The base flow usages are also from
    //     the Flows library model: ... successionFlows for a succession flow", 7.16.2)
    // constraint: SuccessionFlowUsage::checkSuccessionFlowUsageSpecialization,
    //     `specializesFromLibrary('Flows::successionFlows')` (8.3.16.4). An injection, so
    //     sv2-hir's; this layer builds the tree only (ADR-0002).
    pub(super) fn succession_flow_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuccessionFlowUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("succession");
        self.expect_keyword("flow");
        self.flow_declaration();
        self.definition_body();
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
    pub(super) fn at_initial_node_member(&self) -> bool {
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
    // opens is ActionTargetSuccessionMember, read after it by the body: every corpus file
    // that writes it goes on to `then`.
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
    pub(super) fn at_guarded_succession_member(&self) -> bool {
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
    pub(super) fn guarded_succession_member(&mut self) {
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

    pub(super) fn initial_node_item(&mut self) {
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
    pub(super) fn at_action_target_succession_member(&self) -> bool {
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
    /// ActionBodyParameterMember` (8.2.2.17.7) — and so does `KerML`'s
    /// `ConditionalExpression`, `'if' Expression '?' Expression 'else' Expression`
    /// (8.2.5.8.1), which a calculation body may write as its result.
    ///
    /// The expression between the two keywords is scanned rather than parsed: no
    /// expression contains a `then`, and none but a `BodyExpression` reaches a `;`, a `{`
    /// or a `}` without ending, so the first of those four tokens decides. `if i < 0 { }`
    /// is an `IfNode` (see `if_node_body_follows`); `if x ? 1 else 2 }` is the result
    /// expression and left to the body. A guard holding a `BodyExpression`, `if
    /// xs->forAll { ... } then a;`, is declined at its brace, and `if_node_body_follows`
    /// then takes it for an `IfNode`, which reports the `then`; no corpus guard writes one.
    fn at_guarded_target_succession(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "if") && self.scan_for_keyword(n + 1, "then").is_some()
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
    /// `QualifiedName` followed by `OwnedFeatureChain`'s further links (8.2.2.6.5). It
    /// looks past a leading `OwnedCrossMultiplicityMember` in both languages and, in
    /// `SysML` only, by deviation ConnectorEnd-trailing-multiplicity, a trailing
    /// multiplicity, as `connector_end` reads both.
    pub(super) fn skip_connector_end(&self, n: usize) -> Option<usize> {
        let n = self.skip_interface_end(n)?;
        // The trailing multiplicity `connector_end` reads by deviation
        // ConnectorEnd-trailing-multiplicity, so every recogniser sees the same end.
        if self.language == Language::SysMl && self.nth_is(n, SyntaxKind::LBracket) {
            return self.skip_bracketed(n);
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
    pub(super) fn action_target_succession_member(&mut self) {
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
    pub(super) fn guarded_target_succession(&mut self) {
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
    pub(super) fn guard_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::GuardExpressionMember);
        self.expect_keyword("if");
        self.owned_expression();
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
    pub(super) fn at_source_succession_member(&self, body: Body) -> bool {
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
            || self.at_state_usage(n)
            || self.at_exhibit_state_usage(n)
            || self.at_perform_action_usage(n)
            || self.at_assert_constraint_usage(n)
            || self.at_satisfy_requirement_usage(n)
            || self.at_constraint_usage(n)
            || self.at_requirement_usage(n)
            || self.at_concern_usage(n)
            || self.at_viewpoint_usage(n)
            || self.at_calculation_usage(n)
            || self.at_case_usage(n).is_some()
            || self.at_include_use_case_usage(n)
            || self.at_flow_usage(n)
            || self.at_succession_flow_usage(n)
            || self.at_message(n)
            || self.at_connection_usage(n)
            || self.at_interface_usage(n)
            || self.at_allocation_usage(n)
            || self.at_view_usage(n)
            || self.at_event_occurrence_usage(n)
            || self.at_individual_or_portion_usage(n).is_some()
            || self
                .at_simple_usage(n)
                .is_some_and(|usage| usage.class != UsageClass::NonOccurrence)
            || (body.admits_action_body_item() && self.at_action_node(n).is_some())
            || (body == Body::Interface && self.at_default_interface_end(n))
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
    pub(super) fn connector_end_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConnectorEndMember);
        self.connector_end();
        self.finish_node();
    }

    // production: ConnectorEnd
    //
    // ConnectorEnd : ReferenceUsage =
    //     ( ownedRelationship += OwnedCrossMultiplicityMember )?
    //     ( declaredName = NAME REFERENCES )?
    //     ownedRelationship += OwnedReferenceSubsetting             (SysML 8.2.2.13.1)
    //
    // production: OwnedCrossMultiplicityMember
    // production: OwnedCrossMultiplicity
    //
    // OwnedCrossMultiplicityMember : OwningMembership =
    //     ownedRelatedElement += OwnedCrossMultiplicity   (SysML 8.2.2.13.1, KerML 8.2.5.5.1)
    // OwnedCrossMultiplicity : Feature = ownedRelationship += OwnedMultiplicity
    //
    // All three are shared units, read in both languages by `end_reference`: the cross
    // multiplicity's OwnedMultiplicity is each language's own (`owned_multiplicity`),
    // MultiplicityRange in SysML and OwnedMultiplicityRange in KerML. `skip_connector_end`
    // walks the parts in this order.
    //
    // "The identification of a related feature may optionally be preceded by a cross
    // multiplicity and/or an end feature name followed by the keyword references or the
    // symbol ::>" (7.13.2, receipt 5a3a8867).
    //
    // BY DEVIATION ConnectorEnd-trailing-multiplicity (follow_spec_example), a SysML end
    // may also be FOLLOWED by a multiplicity, the end feature's own, as 7.13.2's example
    // writes `[1] hub ::> mainSwitch[1]`: no production admits it and nothing else can
    // stand there, so it is read as an OwnedMultiplicity after the reference and every
    // use is reported (ADR-0022). The corpus writes none.
    fn connector_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConnectorEnd);
        self.end_reference();
        if self.language == Language::SysMl && self.at(SyntaxKind::LBracket) {
            // deviation: ConnectorEnd-trailing-multiplicity
            self.note_deviation(
                "ConnectorEnd-trailing-multiplicity",
                "a multiplicity after a connector end's reference",
            );
            self.owned_multiplicity();
        }
        self.finish_node();
    }

    /// The parts `ConnectorEnd` and `InterfaceEnd` share, `OwnedCrossMultiplicityMember?
    /// ( NAME REFERENCES )? OwnedReferenceSubsetting` (`SysML` 8.2.2.13.1, 8.2.2.14.2),
    /// into the caller's node.
    pub(super) fn end_reference(&mut self) {
        if self.at(SyntaxKind::LBracket) {
            self.start_node(SyntaxKind::OwnedCrossMultiplicityMember);
            self.start_node(SyntaxKind::OwnedCrossMultiplicity);
            self.owned_multiplicity();
            self.finish_node();
            self.finish_node();
        }
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
    /// the scan declines it at its `;` or `{`; `structure_usage_element` reads it first
    /// in any case.
    ///
    /// The prefix skipped is `UsagePrefix`, which is what `succession_as_usage` reads, and
    /// NOT `OccurrenceUsagePrefix`: a recogniser that looked past `snapshot` would accept
    /// a member the parser then cannot consume, and the body loop would ask again for
    /// ever (invariant 3).
    pub(super) fn at_succession_as_usage(&self, n: usize) -> bool {
        let mut n = self.skip_usage_prefix(n);
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
    // ConnectorEnd is read whole, its cross multiplicity included, so `first [1] a then
    // b;` is this production in full.
    //
    // implied specialization: Occurrences::happensBeforeLinks
    // constraint: Succession::checkSuccessionSpecialization, which "requires that a
    //     SuccessionAsUsage specialize the KerML Feature Occurrences::happensBeforeLinks"
    //     (SysML 8.4.9.4, receipt 86e83273). An injection, so sv2-hir's; this layer builds
    //     the tree only (ADR-0002).
    pub(super) fn succession_as_usage(&mut self) {
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
}
