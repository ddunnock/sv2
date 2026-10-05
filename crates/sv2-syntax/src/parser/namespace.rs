// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Namespaces and packages, `KerML` 8.2.3 and `SysML` 8.2.2.5: the root namespace, body
//! elements, memberships, member prefixes, aliases, and element filters.

use rowan::GreenNode;

use crate::counter::{Counter, count};
use crate::diagnostic::Diagnostic;
use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::body::Body;
use crate::parser::case::is_case_head;
use crate::parser::lookahead::{MemberHead, keyword};
use crate::parser::usage::UsageClass;
use crate::parser::usage::is_simple_usage_head;
use crate::parser::{MAX_DEPTH, Parser};

/// What `membership` found after the `MemberPrefix`, which decides the member's node.
#[derive(Clone, Copy)]
pub(super) enum MemberElement {
    /// A package, definition, classifier or annotating element: not a usage.
    Other,
    /// A usage of this class.
    Usage(UsageClass),
    /// An `ActionNode`, which is not a `UsageElement` at all: `ActionBehaviorMember =
    /// BehaviorUsageMember | ActionNodeMember` (`SysML` 8.2.2.17.1), so it is owned
    /// beside the behaviour usages rather than as one of them.
    ActionNode,
}

/// The `SysML` members that open on a keyword, one per recogniser
/// `at_sysml_keyword_member` asks.
#[derive(Clone, Copy, Debug)]
pub(super) enum KeywordMember {
    Definition,
    Action,
    State,
    ExhibitState,
    PerformAction,
    Flow,
    SuccessionFlow,
    Message,
    Connection,
    Interface,
    Allocation,
    View,
    EventOccurrence,
    IndividualOrPortion,
    SuccessionAsUsage,
    BindingConnector,
    AssertConstraint,
    SatisfyRequirement,
    Constraint,
    Requirement,
    Concern,
    Viewpoint,
    Calculation,
    Case,
    IncludeUseCase,
    Simple,
    Reference,
    Extended,
}

/// Every [`KeywordMember`], in the order `at_sysml_keyword_member` asks them. The order
/// is the chain's priority, and ONE LIST: see `at_sysml_keyword_member`.
pub(super) const KEYWORD_MEMBERS: [KeywordMember; 28] = [
    KeywordMember::Definition,
    KeywordMember::Action,
    KeywordMember::State,
    KeywordMember::ExhibitState,
    KeywordMember::PerformAction,
    KeywordMember::Flow,
    KeywordMember::SuccessionFlow,
    KeywordMember::Message,
    KeywordMember::Connection,
    KeywordMember::Interface,
    KeywordMember::Allocation,
    KeywordMember::View,
    KeywordMember::EventOccurrence,
    KeywordMember::IndividualOrPortion,
    KeywordMember::SuccessionAsUsage,
    KeywordMember::BindingConnector,
    KeywordMember::AssertConstraint,
    KeywordMember::SatisfyRequirement,
    KeywordMember::Constraint,
    KeywordMember::Requirement,
    KeywordMember::Concern,
    KeywordMember::Viewpoint,
    KeywordMember::Calculation,
    KeywordMember::Case,
    KeywordMember::IncludeUseCase,
    KeywordMember::Simple,
    KeywordMember::Reference,
    KeywordMember::Extended,
];

impl KeywordMember {
    /// Whether this member's recogniser can accept at a member with this `head`.
    ///
    /// Each recogniser reads only prefix words and `#` metadata before the keyword that
    /// decides it, which is therefore the head; the keyword is the one its recogniser
    /// tests, named at each arm or, for `Simple` and `Case`, read from the table that
    /// recogniser walks. The three that decide on a prefix are admitted by the
    /// flag `member_head` keeps for it. An `end` head admits everything: an
    /// `EndUsagePrefix` owns a cross feature (8.2.2.6.2) that `member_head` does not
    /// look past, and every usage recogniser reads one. The
    /// `every_keyword_member_is_admitted_where_it_accepts` test holds each arm to its
    /// recogniser.
    pub(super) fn admits(self, head: MemberHead<'_>) -> bool {
        if head.word == Some("end") {
            return true;
        }
        let is = |words: &[&str]| head.word.is_some_and(|word| words.contains(&word));
        match self {
            Self::Definition => Parser::opens_definition(head),
            Self::Action => is(&["action"]),
            Self::State => is(&["state"]),
            Self::ExhibitState => is(&["exhibit"]),
            Self::PerformAction => is(&["perform"]),
            Self::Flow => is(&["flow"]),
            Self::SuccessionFlow => is(&["succession"]),
            Self::Message => is(&["message"]),
            Self::Connection => is(&["connection", "connect"]),
            Self::Interface => is(&["interface"]),
            Self::Allocation => is(&["allocation", "allocate"]),
            Self::View => is(&["view"]),
            Self::EventOccurrence => is(&["event"]),
            // `individual` or a `PortionKind`, both skipped by `member_head`.
            Self::IndividualOrPortion => head.after_occurrence_word,
            Self::SuccessionAsUsage => is(&["succession", "first"]),
            Self::BindingConnector => is(&["binding", "bind"]),
            Self::AssertConstraint => is(&["assert"]),
            // `'assert'? 'not'? 'satisfy'`: whichever is written first.
            Self::SatisfyRequirement => is(&["assert", "not", "satisfy"]),
            Self::Constraint => is(&["constraint"]),
            Self::Requirement => is(&["requirement"]),
            Self::Concern => is(&["concern"]),
            Self::Viewpoint => is(&["viewpoint"]),
            Self::Calculation => is(&["calc"]),
            Self::Case => head.word.is_some_and(is_case_head),
            Self::IncludeUseCase => is(&["include"]),
            Self::Simple => head.word.is_some_and(is_simple_usage_head),
            // `ref` is the kind keyword, and `member_head` skips it as a prefix word.
            Self::Reference => head.after_ref,
            // At least one `#` skipped on the way to the head. The recogniser decides the
            // rest, that no kind keyword follows it.
            Self::Extended => head.after_metadata,
        }
    }
}

impl Parser<'_> {
    /// Whether the keyword deciding which `PackageBodyElement` this is, is `text`.
    ///
    /// Looks past an optional `VisibilityIndicator`. `MemberPrefix`'s visibility is
    /// optional and `Import`'s is required, so the indicator never decides on its
    /// own — the keyword after it does.
    pub(super) fn at_element_keyword(&self, text: &str) -> bool {
        self.nth_is_keyword(usize::from(self.at_visibility()), text)
    }

    /// Whether a `Package` starts at the `n`th meaningful token: its
    /// `PrefixMetadataMember*` (`SysML` 8.2.2.5.1) looked past, then `package`.
    pub(super) fn at_package(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_prefix_metadata(n), "package")
    }

    /// Whether a `LibraryPackage` starts at the `n`th meaningful token: `'standard'?
    /// 'library'`, its `PrefixMetadataMember*` looked past as `at_package` looks past
    /// them, then `package` (`SysML` 8.2.2.5.1, `KerML` 8.2.5.13). The `standard` is
    /// optional by deviation `LibraryPackage` (`follow_xtext`).
    pub(super) fn at_library_package(&self, n: usize) -> bool {
        let k = n + usize::from(self.nth_is_keyword(n, "standard"));
        self.nth_is_keyword(k, "library") && self.at_package(k + 1)
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
    /// Every one of `NonFeatureElement`'s alternatives is implemented: `Package` and
    /// `LibraryPackage`, shared units, the same productions in both grammars, and
    /// `KerML`'s own `Dependency`, `Namespace`, `Type`, the classifiers, `Function`,
    /// `Predicate`, `Multiplicity` and the nine relationship declarations. All ten of
    /// `FeatureElement`'s are.
    ///
    /// This is the check that stops a `SysML` construct being read out of a `KerML`
    /// file. `part def` is not reachable from `NamespaceBodyElement`, so a `.kerml` file
    /// containing one must not parse — which it silently did before ADR-0014 was
    /// implemented here, because one root was applied to both file kinds.
    fn at_member_element(&self, n: usize) -> bool {
        count(Counter::MemberDispatch);
        // Both grammars reach AnnotatingElement from their member, by different routes:
        // MemberElement = AnnotatingElement | NonFeatureElement in KerML 8.2.3.4.1, and
        // DefinitionElement's third alternative in SysML 8.2.2.6.1. So it is admitted in
        // every body either language has, and is asked before the languages part.
        if self.at_annotating_member(n) {
            return true;
        }
        match self.language {
            Language::KerMl => {
                self.at_kerml_non_feature_element(n)
                    || self.at_feature(n)
                    || self.at_kerml_keyword_feature_element(n)
            }
            Language::SysMl => {
                self.at_sysml_keyword_member(n)
                    // Only when no keyword usage starts here; see `membership`.
                    || self.at_default_reference_usage(n)
            }
        }
    }

    /// Whether a `SysML` member that opens on a KEYWORD starts at the `n`th token.
    ///
    /// Every member `at_member_element` admits in `SysML` except `DefaultReferenceUsage`,
    /// the one that opens on a bare name. Split out because a calculation body asks the
    /// same question — "can an item start here, or is this the result expression?" — and
    /// a keyword is never an expression (8.2.2.1.2), so the keyword members are exactly
    /// the items it can decide on at once. ONE LIST, so that a production added here is
    /// an item of a calculation body too. When `at_result_expression` kept a list of its
    /// own, `SuccessionAsUsage`, `BindingConnectorAsUsage`, `AssertConstraintUsage` and
    /// `CalculationUsage` were each added here and not there, and `first a then b;` in a
    /// `constraint def` body was read as the start of its result expression.
    pub(super) fn at_sysml_keyword_member(&self, n: usize) -> bool {
        count(Counter::KeywordMemberDispatch);
        // Each member is asked only where its head admits it (roadmap Phase 5), and in
        // `KEYWORD_MEMBERS`'s order, which is this chain's priority.
        let head = self.member_head(n);
        KEYWORD_MEMBERS
            .iter()
            .any(|&member| member.admits(head) && self.at_keyword_member(member, n))
    }

    /// Whether `member` starts at the `n`th token: its recogniser, by name.
    pub(super) fn at_keyword_member(&self, member: KeywordMember, n: usize) -> bool {
        match member {
            KeywordMember::Definition => self.at_definition_element(n),
            KeywordMember::Action => self.at_action_usage(n),
            KeywordMember::State => self.at_state_usage(n),
            KeywordMember::ExhibitState => self.at_exhibit_state_usage(n),
            KeywordMember::PerformAction => self.at_perform_action_usage(n),
            KeywordMember::Flow => self.at_flow_usage(n),
            KeywordMember::SuccessionFlow => self.at_succession_flow_usage(n),
            KeywordMember::Message => self.at_message(n),
            KeywordMember::Connection => self.at_connection_usage(n),
            KeywordMember::Interface => self.at_interface_usage(n),
            KeywordMember::Allocation => self.at_allocation_usage(n),
            KeywordMember::View => self.at_view_usage(n),
            KeywordMember::EventOccurrence => self.at_event_occurrence_usage(n),
            KeywordMember::IndividualOrPortion => self.at_individual_or_portion_usage(n).is_some(),
            KeywordMember::SuccessionAsUsage => self.at_succession_as_usage(n),
            KeywordMember::BindingConnector => self.at_binding_connector_as_usage(n),
            KeywordMember::AssertConstraint => self.at_assert_constraint_usage(n),
            KeywordMember::SatisfyRequirement => self.at_satisfy_requirement_usage(n),
            KeywordMember::Constraint => self.at_constraint_usage(n),
            KeywordMember::Requirement => self.at_requirement_usage(n),
            KeywordMember::Concern => self.at_concern_usage(n),
            KeywordMember::Viewpoint => self.at_viewpoint_usage(n),
            KeywordMember::Calculation => self.at_calculation_usage(n),
            KeywordMember::Case => self.at_case_usage(n).is_some(),
            KeywordMember::IncludeUseCase => self.at_include_use_case_usage(n),
            KeywordMember::Simple => self.at_simple_usage(n).is_some(),
            KeywordMember::Reference => self.at_reference_usage(n),
            KeywordMember::Extended => self.at_extended_usage(n),
        }
    }

    // production: RootNamespace@kerml
    // production: RootNamespace@sysml
    //
    // RootNamespace = PackageBodyElement*                          (SysML 8.2.2.5.1)
    // RootNamespace = NamespaceBodyElement*                        (KerML 8.2.3.4.1)
    //
    // The one production the two grammars state differently at the start symbol, which
    // is what ADR-0014 exists for. `Body::Root` carries the difference; the loop below
    // is shared, because everything else about reading a body is. Two grammar units, and
    // this reads both bodies, so it carries both markers.
    pub(super) fn root_namespace(mut self) -> (GreenNode, Vec<Diagnostic>, Vec<Diagnostic>) {
        self.start_node(SyntaxKind::RootNamespace);
        self.body_elements(None, Body::Root);
        // Trailing trivia belongs to the tree as much as anything else.
        self.eat_trivia();
        self.finish_node();
        (self.builder.finish(), self.errors, self.deviations)
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
    pub(super) fn body_elements(&mut self, until: Option<SyntaxKind>, body: Body) {
        while self.at_bare_comment_member()
            || (!self.at_end() && !until.is_some_and(|kind| self.at(kind)))
        {
            let start = self.pos;
            if !self.body_element(body) {
                return;
            }
            if self.pos == start {
                // No arm consumed a token. Every recogniser in `body_element` must agree
                // with the production it dispatches to, and when one does not — it
                // accepts, the production reads nothing and `expect_*` records an error
                // without consuming — the loop asks the same question at the same token
                // for ever, and each pass adds a diagnostic until the process runs out of
                // memory. Taking one token turns such a disagreement into one diagnostic
                // (invariant 3); the disagreement itself is still the defect to fix.
                self.error_token();
            }
        }
    }

    /// One element of `body`, as `body_elements` describes them. Returns `false` when
    /// what is left is the body's trailing result expression, which ends the item run.
    // production: NamespaceBodyElement@kerml
    // production: NamespaceMember@kerml
    // production: PackageBodyElement@sysml
    // production: DefinitionBodyItem@sysml
    // production: RequirementBodyItem@sysml
    // production: ViewDefinitionBodyItem@sysml
    // production: ViewBodyItem@sysml
    // production: NonBehaviorBodyItem@sysml
    // production: ActionBodyItem@sysml
    // production: CalculationBodyItem@sysml
    // production: CaseBodyItem@sysml
    // production: StateBodyItem@sysml
    //
    // Every body's item production is an alternation over memberships, and this one
    // dispatcher reads each of them whole, parameterised by `body`: `Body::member` names
    // the membership a member is owned through, and the `admits_*` questions which of a
    // production's extra alternatives the body has. The productions are quoted at each
    // `Body` variant, and the alternations build no node: the membership says which. Two
    // tests hold the whole matrix, every alternative tried in every body, admitted with
    // its membership exactly where its production lists it and reported elsewhere:
    // `every_body_admits_exactly_its_item_production` (SysML) and
    // `every_kerml_body_admits_exactly_its_elements` (KerML).
    //
    // NonBehaviorBodyItem is reached as the first alternative of ActionBodyItem (8.2.2.17.1)
    // and StateBodyItem (8.2.2.18.1), never alone; NamespaceMember is
    // `NonFeatureMember | NamespaceFeatureMember` (KerML 8.2.3.4.1), the first read by
    // `membership`, the second by `kerml_feature_item`.
    fn body_element(&mut self, body: Body) -> bool {
        count(Counter::MemberDecisions);
        if self.depth >= MAX_DEPTH {
            // Too deeply nested to recurse into another body. Recover one token
            // at a time, exactly as unrecognised text is recovered: every byte
            // still reaches the tree, and the stack does not grow (invariant 3).
            self.report_too_deep();
            self.error_token();
        } else if self.at_bare_comment_member() {
            // A Comment member, AnnotatingElement being a MemberElement in KerML
            // (8.2.3.4.3) and a DefinitionElement in SysML (8.2.2.5.2). Before the result
            // expression too: `calc def C { /* c */ x + 1 }` is an item, then the result.
            self.bare_comment_member(body);
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
        } else if self.language == Language::KerMl
            && body.ends_in_result_expression()
            && self.at_result_expression()
        {
            // A KerML function body's item run is over, and what is left is its result
            // expression. Asked BEFORE the features, because a keywordless feature opens
            // on a name as `age >= 35` does, and the feature branch would take it.
            return false;
        } else if self.language == Language::KerMl && self.kerml_feature_item(body) {
            // Read by the call, which answers whether it read one.
        } else if body == Body::Interface && self.at_usage_no_interface_body_admits() {
            // InterfaceNonOccurrenceUsageElement lists ReferenceUsage, AttributeUsage,
            // EnumerationUsage, BindingConnectorAsUsage and SuccessionAsUsage and no
            // other (8.2.2.14.1): a keywordless usage, or one declared by `#` alone, is no
            // item of an interface body. Reported and recovered over as a whole.
            self.recover_statement();
        } else if self.body_specific_item(body) {
            // GuardedSuccessionMember, InitialNodeMember, ReturnParameterMember,
            // VariantUsageMember, RequirementConstraintMember,
            // RequirementVerificationMember, SubjectMember, ActorMember, ObjectiveMember
            // and TransitionUsageMember: see
            // `body_specific_item`.
        } else if body.ends_in_result_expression() && self.at_result_expression() {
            // The item run is over and what is left is the body's trailing
            // expression, which is not a member. `calculation_body_part` reads it;
            // the loop must not recover over it one token at a time.
            return false;
        } else if body.admits_source_succession() && self.at_source_succession_member(body) {
            // `SourceSuccessionMember? <occurrence usage member>`, in whichever of
            // three item productions this body has; see `source_succession_item`.
            self.source_succession_item(body);
        } else if self.at_member_element(usize::from(self.at_visibility()))
            || (body.admits_action_body_item()
                && self
                    .at_action_node(usize::from(self.at_visibility()))
                    .is_some())
        {
            // An action node is an ActionNodeMember, ActionBehaviorMember's second
            // alternative (8.2.2.17.1), and so an item of the action-body family only:
            // no other body's item production reaches ActionNode, which
            // validateControlNodeOwningType states of the metaclass too (8.3.17.6,
            // receipt 695df335). Hence here, with the body in hand, and not in the
            // body-agnostic `at_member_element`.
            let element = self.membership(body);
            self.behaviour_targets(body, element);
        } else {
            self.recover_statement();
        }
        true
    }

    /// The items that own their element through a membership of their own, each told by a
    /// reserved keyword and admitted by the body's item production alone. Returns whether
    /// one was read.
    ///
    /// Split out of `body_element`, which asks it before the result-expression test: every
    /// one of these continues an item run, and none is admitted by a calculation body
    /// except the four that are (the guarded succession, `first`, `return`, `variant`),
    /// nor by a case body except those four and `subject`, `actor` and `objective`, so the others'
    /// position relative to that test decides nothing. The arms are disjoint on their keywords, so
    /// their order decides nothing either.
    fn body_specific_item(&mut self, body: Body) -> bool {
        if body.admits_action_body_item() && self.at_guarded_succession_member() {
            // ActionBodyItem's fourth alternative (SysML 8.2.2.17.1). An item of its
            // own with no suffix, so it is not `initial_node_item`'s shape: nothing
            // follows it here, and tests/rejection/guarded-succession-takes-no-target-succession.sysml
            // holds that. Disjoint from the initial node below on the token after the
            // source name, so the order of the two arms decides nothing.
            self.guarded_succession_member();
        } else if body.admits_action_body_item() && self.at_initial_node_member() {
            // ActionBodyItem's second alternative (SysML 8.2.2.17.1), reached from an
            // action body and, through CalculationBodyItem (8.2.2.19), a calculation
            // body. Owns its membership as ReturnParameterMember below does, so it
            // cannot go through `membership`. Before the result-expression test for
            // the same reason `return` is: it continues the item run.
            self.initial_node_item();
        } else if body.admits_return_parameter() && self.at_return_parameter_member() {
            // CalculationBodyItem's second alternative (SysML 8.2.2.19). Before the
            // result-expression test below, because `return` is where the item run
            // continues rather than where it ends; `at_result_expression` says so
            // too, so neither position depends on the other being right.
            if body == Body::Case {
                // 8.2.2.22's CaseBodyItem has no ReturnParameterMember; see
                // `Body::admits_return_parameter`.
                // deviation: CaseBodyItem
                self.note_deviation("CaseBodyItem", "a return parameter in a case body");
            }
            self.return_parameter_member();
        } else if body.admits_variant() && self.at_element_keyword("variant") {
            // DefinitionBodyItem's and NonBehaviorBodyItem's VariantUsageMember
            // (SysML 8.2.2.6.1, 8.2.2.17.1). Owns its element through a VariantMembership
            // of its own, so it cannot go through `membership`. `variant` is reserved,
            // so it decides; before the result-expression test, as `return` is, because
            // it continues the item run.
            self.variant_usage_member();
        } else if self.requirement_body_member(body) {
            // SubjectMember, RequirementConstraintMember, FramedConcernMember,
            // RequirementVerificationMember, ActorMember and StakeholderMember: see
            // `requirement_body_member`.
        } else if body.admits_expose() && self.at_keyword("expose") {
            // ViewBodyItem's Expose (SysML 8.2.2.26.2). `at_keyword`, not
            // `at_element_keyword`: an Expose writes no visibility, so a `private` before
            // it is no item and is recovered over.
            self.expose();
        } else if body.admits_render() && self.at_element_keyword("render") {
            // ViewDefinitionBodyItem's and ViewBodyItem's ViewRenderingMember (SysML
            // 8.2.2.26.1, .2): a ViewRenderingMembership of its own, so not `membership`'s.
            self.view_rendering_member();
        } else if body.admits_objective() && self.at_element_keyword("objective") {
            // CaseBodyItem's fourth alternative (SysML 8.2.2.22), owning its requirement
            // through an ObjectiveMembership of its own, as SubjectMember does.
            self.objective_member();
        } else if body.admits_state_action()
            && ["entry", "do", "exit"]
                .iter()
                .any(|word| self.at_element_keyword(word))
        {
            // StateBodyItem's fourth, fifth and sixth alternatives (SysML 8.2.2.18.1),
            // each owning a StateActionUsage through a StateSubactionMembership of its
            // own; an entry action takes its EntryTransitionMembers after it.
            self.state_action_item();
        } else if body.admits_transition() && self.at_element_keyword("transition") {
            // StateBodyItem's third alternative (SysML 8.2.2.18.1). An item of its own,
            // owning its element through a membership of its own. A `transition` that
            // opens a TARGET transition is read as a suffix by `behaviour_targets` and
            // never reaches here unless nothing precedes it, where it is no item and
            // `transition_usage` reports the missing source.
            self.transition_usage_member();
        } else {
            return false;
        }
        true
    }

    /// Whether a KEYWORD that begins a member is written here.
    ///
    /// The recogniser is the body loop's, minus everything that opens on a bare NAME: a
    /// `ReferenceUsage` needs its `ref`, and `DefaultReferenceUsage` is excluded outright,
    /// because a NAME is exactly what a run of unreadable text is full of. Asked only
    /// while recovering, where the question is "does a new member start here", not "which
    /// member is it".
    pub(super) fn at_member_keyword(&self) -> bool {
        self.at_import()
            || self.at_element_keyword("alias")
            || self.at_element_keyword("filter")
            || self.at_annotating_member(0)
            || match self.language {
                Language::KerMl => {
                    self.at_package(0)
                        || self.at_library_package(0)
                        || self.at_dependency(0)
                        || self.at_classifier(0).is_some()
                        || self.at_kerml_keyword_feature_element(0)
                        || self.at_relationship_declaration(0).is_some()
                        || self.at_kerml_type(0)
                        || self.at_kerml_function(0).is_some()
                        || self.at_kerml_namespace(0)
                        || self.nth_is_keyword(0, "multiplicity")
                }
                Language::SysMl => {
                    self.at_definition_element(0)
                        // A body item rather than a member `membership` reads, and SysML's
                        // alone: KerML has no variants.
                        || self.at_element_keyword("variant")
                        || self.at_simple_usage(0).is_some()
                        || self.at_action_usage(0)
                        || self.at_state_usage(0)
                        || self.at_exhibit_state_usage(0)
                        // StateBodyItems rather than members `membership` reads.
                        || ["transition", "entry", "do", "exit"]
                            .iter()
                            .any(|word| self.at_element_keyword(word))
                        || self.at_perform_action_usage(0)
                        || self.at_flow_usage(0)
                        || self.at_succession_flow_usage(0)
                        || self.at_connection_usage(0)
                        || self.at_succession_as_usage(0)
                        || self.at_binding_connector_as_usage(0)
                        || self.at_assert_constraint_usage(0)
                        || self.at_constraint_usage(0)
                        || self.at_requirement_usage(0)
                        || self.at_calculation_usage(0)
                        || self.at_case_usage(0).is_some()
                        || self.at_include_use_case_usage(0)
                        // Body items rather than members `membership` reads.
                        || self.at_element_keyword("verify")
                        || self.at_element_keyword("actor")
                        || self.at_element_keyword("objective")
                        || self.at_action_node(0).is_some()
                }
            }
    }

    /// The suffix after a behaviour usage: `ActionTargetSuccessionMember*` in an action
    /// body, `TargetTransitionUsageMember*` in a state body.
    ///
    /// `ActionBodyItem`'s third alternative is `SourceSuccessionMember?
    /// ActionBehaviorMember ActionTargetSuccessionMember*` (8.2.2.17.1), so the `then X;`
    /// members belong to the item just read when — and only when — it was an
    /// `ActionBehaviorMember`, a `BehaviorUsageMember` or an `ActionNodeMember`; the
    /// `NonBehaviorBodyItem` alternative that reads structure usages takes no such
    /// suffix, so `part p; then b;` leaves the `then` reported.
    pub(super) fn behaviour_targets(&mut self, body: Body, element: MemberElement) {
        if body == Body::State && matches!(element, MemberElement::Usage(UsageClass::Behavior)) {
            // StateBodyItem: `SourceSuccessionMember? BehaviorUsageMember
            // TargetTransitionUsageMember*` (8.2.2.18.1). A target transition's source is
            // "the closest lexically previous state usage" (7.18.3, receipt 6e6e9493),
            // connected in resolution.
            while self.at_target_transition_usage_member() {
                self.target_transition_usage_member();
            }
            return;
        }
        if body.admits_action_body_item()
            && matches!(
                element,
                MemberElement::Usage(UsageClass::Behavior) | MemberElement::ActionNode
            )
        {
            while self.at_action_target_succession_member() {
                self.action_target_succession_member();
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
    pub(super) fn alias_member(&mut self) {
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
    // NamespaceFeatureMember, its sibling, is marked at `namespace_feature_member`.
    //
    // production: NonOccurrenceUsageMember
    // production: OccurrenceUsageMember
    // production: StructureUsageMember
    // production: BehaviorUsageMember
    //
    // <kind>UsageMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += <kind>UsageElement    (SysML 8.2.2.6.1)
    //
    // The four usage memberships, with the same shape again. Which one a member is
    // depends on the element after the MemberPrefix, so the node is opened at a
    // checkpoint once the element has said what it is — `Body::member` maps the two to
    // the node. Each is marked as the member it is, as DefinitionMember is; the
    // <kind>UsageElement alternations are marked at the methods that read them
    // (`usage_element_of_class` and its class methods). BehaviorUsageMember is reached only from ActionBodyItem's third
    // alternative, whose `then` prefix and trailing target successions its callers read.
    //
    // production: ActionNodeMember@sysml
    //
    // ActionNodeMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += ActionNode            (SysML 8.2.2.17.1)
    //
    // The same shape once more, marked as the member it is; ActionNode is marked at
    // `action_node`.
    //
    // production: ActionBehaviorMember@sysml
    //
    // ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember   (SysML 8.2.2.17.1)
    //
    // An alternation with no node, marked because both of its alternatives are read —
    // the convention OwnedExpression follows. Which one was taken is the member node.
    // production: MemberElement@kerml
    // production: DefinitionElement@sysml
    //
    // MemberElement = AnnotatingElement | NonFeatureElement          (KerML 8.2.3.4.3)
    // DefinitionElement = Package | LibraryPackage | AnnotatingElement | Dependency
    //     | the twenty-five definitions                            (SysML 8.2.2.5.2)
    //
    // Both alternations, read here: the annotating element first in either language, a
    // bare REGULAR_COMMENT among them (see `at_bare_comment_member`); then KerML's
    // NonFeatureElement whole, by `kerml_non_feature_element`; and SysML's packages here
    // and the rest by `definition_element`. No node: the element says which.
    pub(super) fn membership(&mut self, body: Body) -> MemberElement {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.member_prefix();
        let mut element = MemberElement::Other;
        if self.at_annotating_member(0) {
            self.annotating_element();
        } else if self.language == Language::KerMl {
            // MemberElement's other alternative (KerML 8.2.3.4.3). The guard is here
            // rather than left to `at_member_element`'s caller, because a dispatch that
            // is only correct when reached one way is a trap: every classifier unit is
            // scoped `kerml`, and SysML reaches DefinitionElement instead, so `class
            // Foo;` in a .sysml file is text SysML does not state.
            if !self.kerml_non_feature_element() {
                self.error_expected("a package, a classifier or a dependency");
            }
        } else if self.at_package(0) {
            self.package();
        } else if self.at_library_package(0) {
            self.library_package();
        } else if let Some(node) = self
            .at_action_node(0)
            .filter(|_| body.admits_action_body_item())
        {
            // Only the action-body family reaches ActionNodeMember; `body_elements`
            // asks the same question before calling here. The guard is repeated for
            // the reason the classifier one above is: a dispatch that is only correct
            // when reached one way is a trap. BEFORE the usages, because `action publish
            // send x;` opens as an ActionUsage does and is not one: the `send` after the
            // declaration is what makes it a SendNode.
            self.action_node(node);
            element = MemberElement::ActionNode;
        } else if body == Body::Interface && self.at_default_interface_end(0) {
            // InterfaceOccurrenceUsageElement's first alternative (8.2.2.14.1). Its class
            // is neither Structure nor Behavior, and `Body::member` owns every
            // occurrence class through InterfaceOccurrenceUsageMember alike; Structure is
            // the one that takes no target successions after it, as an end takes none.
            self.default_interface_end();
            element = MemberElement::Usage(UsageClass::Structure);
        } else if self.definition_element() {
            // A definition: `MemberElement::Other`, which `element` already is.
        } else if let Some(class) = self.usage_element_of_class() {
            element = MemberElement::Usage(class);
        } else {
            self.error_expected("a package, a part definition or a usage");
        }
        self.start_node_at(start, body.member(self.language, element));
        self.finish_node();
        element
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

    // production: MemberPrefix
    //
    // MemberPrefix : Membership = ( visibility = VisibilityIndicator )?
    //
    // The node is built whether or not a visibility is there. An empty one is the
    // honest shape: the slot exists in the production, and a tree that omits the
    // node when the slot is empty makes every consumer handle two shapes for one
    // construct.
    pub(super) fn member_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MemberPrefix);
        if self.at_visibility() {
            self.visibility_indicator();
        }
        self.finish_node();
    }

    // production: Package
    //
    // Package = ( ownedRelationship += PrefixMetadataMember )*
    //           PackageDeclaration PackageBody                   (SysML 8.2.2.5.1)
    //
    // The PrefixMetadataMembers stand directly in the package, with no extension-keyword
    // node between, as the clause writes them. KerML's Package states the same text over
    // its own PrefixMetadataMember (8.2.5.13, 8.2.5.12), which `prefix_metadata_member`
    // reads in the file's language.
    pub(super) fn package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Package);
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
        self.package_declaration();
        self.package_body();
        self.finish_node();
    }

    // production: LibraryPackage
    //
    // LibraryPackage =
    //     ( isStandard ?= 'standard' ) 'library'
    //     ( ownedRelationship += PrefixMetadataMember )*
    //     PackageDeclaration PackageBody          (SysML 8.2.2.5.1, KerML 8.2.5.13)
    //
    // Read as deviation LibraryPackage (follow_xtext) writes it: `'standard'?`. The printed
    // group has no `?`, which would make every library package standard, where KerML 7.4.14
    // writes `library package AddressBooks {` and the LibraryPackage metaclass says
    // isStandard "should only be set to true" for recognised standard libraries
    // (8.3.4.13.3). So `library package` without `standard` is the text the deviation adds,
    // and it carries the note; the corpus writes nothing else.
    //
    // One unit in both grammars, whose texts are the same (ADR-0015). The
    // PrefixMetadataMember* is each language's own, PrefixMetadataMember@sysml or
    // PrefixMetadataMember@kerml, read by `prefix_metadata_member`.
    pub(super) fn library_package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::LibraryPackage);
        if self.at_keyword("standard") {
            self.bump_as(keyword("standard").unwrap_or(SyntaxKind::BasicName));
        } else {
            // deviation: LibraryPackage
            self.note_deviation("LibraryPackage", "a library package that is not `standard`");
        }
        self.expect_keyword("library");
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
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
    pub(super) fn identification(&mut self) {
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

    // production: PackageBody@kerml
    // production: PackageBody@sysml
    //
    // PackageBody = ';' | '{' PackageBodyElement* '}'              (SysML 8.2.2.5.1)
    // PackageBody = ';' | '{' ( NamespaceBodyElement
    //                         | ElementFilterMember )* '}'         (KerML 8.2.5.13)
    //
    // Two grammar units, one method: `Body::Package` asks `body_elements` for the
    // language's own items, and admits ElementFilterMember in both, where both grammars
    // put it. Marked on DefinitionBody's convention — the body is read in full, its item
    // productions only in part.
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
