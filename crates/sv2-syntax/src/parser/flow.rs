// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Flows and messages, `SysML` 8.2.2.13, and payload features.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;

impl Parser<'_> {
    /// Whether a `FlowUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'flow'` with no `def` after it (`SysML` 8.2.2.16): the
    /// `def` is what makes it the `FlowDefinition` beside it, as for every usage.
    pub(super) fn at_flow_usage(&self, n: usize) -> bool {
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
    pub(super) fn flow_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("flow");
        self.flow_declaration();
        self.definition_body();
        self.finish_node();
    }

    /// Whether a `Message` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'message'` (`SysML` 8.2.2.16). No `def` test, as for
    /// `perform` and `event`: there is no message definition, a message being defined by
    /// flow definitions (7.16.2).
    pub(super) fn at_message(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "message")
    }

    // production: Message@sysml
    //
    // Message : FlowUsage =
    //     OccurrenceUsagePrefix 'message'
    //     MessageDeclaration DefinitionBody
    //     { isAbstract = true }                                  (SysML 8.2.2.16)
    //
    // "A flow usage is declared as a message using the kind keyword message rather than
    // flow ... A message is always abstract (whether or not the abstract keyword is
    // included explicitly in its declaration)" (7.16.2, receipt 13d6f883). The metaclass is
    // FlowUsage (8.3.16.3, receipt 92bc5ec5), as FlowUsage's own is; `isAbstract = true` is
    // an assignment the text does not carry, so the tree records only what was written.
    // The Pilot factors the keyword into MessageKeyword; deviation MessageKeyword
    // (xtext_only, follow_spec) says to match the literal, so there is no production for it.
    //
    // A StructureUsageElement (8.2.2.6.4), as FlowUsage is.
    //
    // implied specialization: Flows::messages ("The base flow usages are also from the
    //     Flows library model: messages for a message", 7.16.2)
    // constraint: FlowUsage::checkFlowUsageSpecialization, `specializesFromLibrary(
    //     'Flows::messages')` (8.3.16.3). An injection, so sv2-hir's (ADR-0002). A message
    //     has no end features, so checkFlowUsageFlowSpecialization does not bite.
    pub(super) fn message(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Message);
        self.occurrence_usage_prefix();
        self.expect_keyword("message");
        self.message_declaration();
        self.definition_body();
        self.finish_node();
    }

    // production: MessageDeclaration@sysml
    //
    // MessageDeclaration : FlowUsage =
    //       UsageDeclaration ValuePart?
    //       ( 'of' ownedRelationship += FlowPayloadFeatureMember )?
    //       ( 'from' ownedRelationship += MessageEventMember
    //         'to' ownedRelationship += MessageEventMember
    //       )?
    //     | ownedRelationship += MessageEventMember 'to'
    //       ownedRelationship += MessageEventMember              (SysML 8.2.2.16)
    //
    // FlowDeclaration's shape with MessageEventMembers for its FlowEndMembers, and told
    // apart the same way: both alternatives may open on a NAME, and only the second writes
    // `to` straight after a whole reference, so the reference is looked past and the token
    // after it decides. A MessageEvent is an OwnedReferenceSubsetting, a QualifiedName or a
    // chain of them, which is the dotted run `flow_end_segments` walks; unlike a FlowEnd its
    // segment count builds nothing different, so deviation FlowEndSubsetting is not
    // FlowEnd's to lend here.
    //
    // The Pilot writes `UsageDeclaration?` (SysML.xtext:1250), which changes no text accepted
    // since a UsageDeclaration may be empty, and `PayloadFeatureMember`, which DOES: its
    // `Payload` fragment (SysML.xtext:1302) has a fourth alternative, `Identification?
    // ValuePart`, that the specification's PayloadFeature (8.2.2.16) lacks, so `of p = x`
    // is the Pilot's and not the specification's. Deviation Payload (xtext_only,
    // follow_spec) settles it, and `flow_payload_feature_member` reads the specification's
    // three alternatives, as it does for FlowDeclaration.
    //
    // 7.16.2 says the value is written "if the source and target event identification is
    // not included", but the production admits both together, and the grammar decides.
    fn message_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MessageDeclaration);
        let events_first = self
            .flow_end_segments(0)
            .is_some_and(|(after, _)| self.nth_is_keyword(after, "to"));
        if events_first {
            self.message_event_member();
            self.expect_keyword("to");
            self.message_event_member();
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
                self.message_event_member();
                self.expect_keyword("to");
                self.message_event_member();
            }
        }
        self.finish_node();
    }

    // production: MessageEventMember@sysml
    //
    // MessageEventMember : ParameterMembership =
    //     ownedRelatedElement += MessageEvent                    (SysML 8.2.2.16)
    //
    // production: MessageEvent@sysml
    //
    // MessageEvent : EventOccurrenceUsage =
    //     ownedRelationship += OwnedReferenceSubsetting          (SysML 8.2.2.16)
    //
    // An EventOccurrenceUsage (8.3.9.2, receipt 9cf02679) written as `event`'s reference
    // alternative less its `event` and its FeatureSpecializationPart: "a message declaration
    // may identify the source and target events at which a transfer may be initiated and
    // received, respectively" (7.16.2). No multiplicity, so `from a[1]` is reported.
    //
    // constraint: EventOccurrenceUsage::validateEventOccurrenceUsageReference (8.3.9.2):
    //     the reference's target is an OccurrenceUsage. sv2-resolve's.
    fn message_event_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MessageEventMember);
        self.eat_trivia();
        self.start_node(SyntaxKind::MessageEvent);
        self.owned_reference_subsetting();
        self.finish_node();
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
    pub(super) fn flow_declaration(&mut self) {
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
    pub(super) fn flow_end_segments(&self, n: usize) -> Option<(usize, usize)> {
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
    pub(super) fn flow_end_member(&mut self) {
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
        if self.language == Language::KerMl {
            self.kerml_flow_end();
            return;
        }
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowEnd);
        match self.flow_end_segments(0) {
            None => self.error_expected("a flow end"),
            Some((_, 1)) => self.flow_feature_member(),
            Some((_, 2)) => {
                // deviation: FlowEndSubsetting
                self.note_deviation("FlowEndSubsetting", "a two-segment flow end's `.`");
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

    /// A `KerML` flow end's `OwnedReferenceSubsetting` of `links` segments: a name, or an
    /// `OwnedFeatureChain` of exactly that many, stopping before the end's last segment.
    pub(super) fn flow_end_subsetting(&mut self, links: usize) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedReferenceSubsetting);
        let start = self.builder.checkpoint();
        self.qualified_name();
        if links > 1 {
            self.start_node_at(start, SyntaxKind::OwnedFeatureChain);
            self.wrap_at(start, &[SyntaxKind::OwnedFeatureChaining]);
            for _ in 1..links {
                self.bump();
                self.owned_feature_chaining();
            }
            self.finish_node();
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
    pub(super) fn flow_feature_member(&mut self) {
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
    pub(super) fn payload_feature(&mut self) {
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
    pub(super) fn payload_feature_is_declared(&self) -> bool {
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
    pub(super) fn payload_feature_specialization_part(&mut self) {
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
}
