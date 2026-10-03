// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Bodies: the [`Body`] each definition and usage reads, and what each one admits.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;
use crate::parser::namespace::MemberElement;
use crate::parser::usage::UsageClass;

/// Which body production is being read, and so which elements it admits.
///
/// The two grammars disagree about the root and about what a package body holds, and
/// the disagreement is not the same shape in both places (ADR-0014):
///
/// ```text
/// RootNamespace@sysml  = PackageBodyElement*                          SysML 8.2.2.5.1
/// PackageBody@sysml    = ';' | '{' PackageBodyElement* '}'             SysML 8.2.2.5.1
/// RootNamespace@kerml  = NamespaceBodyElement*                        KerML 8.2.3.4.1
/// PackageBody@kerml    = ';' | '{' ( NamespaceBodyElement
///                                  | ElementFilterMember )* '}'       KerML 8.2.3.4.1
/// DefinitionBody@sysml = ';' | '{' DefinitionBodyItem* '}'            SysML 8.2.2.6.1
/// ```
///
/// Two questions come out of that, and they do NOT line up, which is why they are asked
/// separately rather than one being derived from the other.
///
/// The membership node differs by language: `PackageBodyElement` reaches `PackageMember`
/// and `NamespaceBodyElement` reaches `NonFeatureMember`.
///
/// Whether a filter is admitted differs by BOTH. An `ElementFilterMember` is a
/// `PackageBodyElement`, so `SysML` admits one at the root and in a package body alike. In
/// `KerML` it is not a `NamespaceBodyElement` at all — `PackageBody` adds it, and the root
/// does not. So a filter is admitted in a `KerML` package body and refused at a `KerML` root,
/// and deriving that from the membership node would accept `filter` where `KerML` has none.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Body {
    /// `RootNamespace`: the whole file.
    Root,
    /// The braced form of `PackageBody`.
    Package,
    /// The braced form of `DefinitionBody`. `SysML` only — `KerML` has no definitions.
    Definition,
    /// The braced form of `RequirementBody`. `SysML` only.
    ///
    /// `RequirementBodyItem = DefinitionBodyItem | SubjectMember | …` (8.2.2.21.1), so
    /// every question `Body::Definition` answers this answers the same way. It is a
    /// variant of its own for the ONE thing it answers differently: the six extra
    /// members, all of them read. A `subject` reached from a
    /// definition body is not a `SubjectMember` — `DefinitionBodyItem` has no such
    /// alternative — and without this variant it would be read as one.
    Requirement,
    /// The braced form of `ActionBody`. `SysML` only.
    ///
    /// ```text
    /// ActionBodyItem = NonBehaviorBodyItem
    ///                | InitialNodeMember ActionTargetSuccessionMember*
    ///                | SourceSuccessionMember? ActionBehaviorMember
    ///                  ActionTargetSuccessionMember*
    ///                | GuardedSuccessionMember                    SysML 8.2.2.17.1
    /// ```
    ///
    /// `NonBehaviorBodyItem`'s implemented alternatives — `Import`, `AliasMember`,
    /// `DefinitionMember` — are the three a definition body reads, so for those this
    /// answers as `Definition` does. It differs in the control-flow alternatives, of
    /// which `InitialNodeMember` is implemented: see `admits_action_body_item`.
    ///
    /// It was a variant before it decided anything, because the ITEM SET differs in the
    /// grammar even where the implemented part did not. That is what let `first` attach
    /// here with no change to the dispatch of anything else — the second time a body
    /// variant argued to be identical to `Definition` stopped being so one production
    /// later, `requirement_body` and `SubjectMember` being the first.
    Action,
    /// The item run inside the braced form of `CalculationBody`. `SysML` only.
    ///
    /// `CalculationBodyItem = ActionBodyItem | ReturnParameterMember` (8.2.2.19), and
    /// `ActionBodyItem`'s first alternative is `NonBehaviorBodyItem = Import |
    /// AliasMember | DefinitionMember | VariantUsageMember | NonOccurrenceUsageMember |
    /// SourceSuccessionMember? StructureUsageMember` (8.2.2.17.1). Of those, the same
    /// three a definition body reads are implemented, which is why this shares the loop.
    ///
    /// It is a variant of its own because the loop must STOP before the trailing
    /// `ResultExpressionMember`, and no other body has one. See `at_result_expression`.
    Calculation,
    /// The braced form of `CaseBody`. `SysML` only.
    ///
    /// ```text
    /// CaseBody     = ';' | '{' CaseBodyItem* ResultExpressionMember? '}'
    /// CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
    ///              | ObjectiveMember                               SysML 8.2.2.22
    /// ```
    ///
    /// `Calculation`'s item run and trailing expression, plus the subject, actor and
    /// objective members: "a case definition or usage is declared as a kind of
    /// calculation definition or usage" (7.22.2, receipt eb25a69f). The BNF leaves
    /// `ReturnParameterMember` out of `CaseBodyItem`, and deviation `CaseBodyItem`
    /// (`follow_xtext`) puts it back, as the 7.23.2 example writes it (receipt 2aa2d6ce);
    /// see `admits_return_parameter`. A variant of its own rather than `Calculation`
    /// because `SubjectMember` and `ObjectiveMember` are items here and not there.
    Case,
    /// The braced form of `StateDefBody` and `StateUsageBody`. `SysML` only.
    ///
    /// ```text
    /// StateBodyItem = NonBehaviorBodyItem
    ///               | SourceSuccessionMember? BehaviorUsageMember
    ///                 TargetTransitionUsageMember*
    ///               | TransitionUsageMember
    ///               | EntryActionMember EntryTransitionMember*
    ///               | DoActionMember | ExitActionMember           SysML 8.2.2.18.1
    /// ```
    ///
    /// Its first alternative is `Action`'s. Its second differs from `ActionBodyItem`'s third
    /// in the member and the suffix: a `BehaviorUsageMember` alone, where
    /// `ActionBehaviorMember` admits an `ActionNodeMember` too, and a TARGET TRANSITION after
    /// it, where an action body takes a target succession. It has no `InitialNodeMember`, no
    /// `ActionNodeMember` and no `GuardedSuccessionMember`, so it is not `Action`, and it
    /// has `TransitionUsageMember`, which nothing else has, and the entry, do and exit
    /// members, which `Body::admits_state_action` answers for.
    State,
    /// The braced form of `InterfaceBody`. `SysML` only.
    ///
    /// ```text
    /// InterfaceBodyItem = DefinitionMember | VariantUsageMember
    ///                   | InterfaceNonOccurrenceUsageMember
    ///                   | SourceSuccessionMember? InterfaceOccurrenceUsageMember
    ///                   | AliasMember | Import                     SysML 8.2.2.14.1
    /// ```
    ///
    /// `DefinitionBodyItem`'s shape with two differences, each of which makes it a
    /// variant: the usage members are its own (`Interface*UsageMember`), and their element
    /// sets are not `NonOccurrenceUsageElement` and `OccurrenceUsageElement`.
    /// `InterfaceNonOccurrenceUsageElement` leaves out `DefaultReferenceUsage` and
    /// `ExtendedUsage`, and `InterfaceOccurrenceUsageElement` adds `DefaultInterfaceEnd`,
    /// so a keywordless `end p : P;` is a port end here and a reference end elsewhere.
    /// See `interface_body_item`.
    Interface,
    /// The braced form of `ViewDefinitionBody`. `SysML` only.
    ///
    /// ```text
    /// ViewDefinitionBodyItem = DefinitionBodyItem | ElementFilterMember
    ///                        | ViewRenderingMember                SysML 8.2.2.26.1
    /// ```
    ///
    /// A superset of `DefinitionBodyItem`, as `RequirementBodyItem` is, so it answers every
    /// question `Definition` answers the same way but two: a `filter` and a `render` are
    /// items here.
    ViewDefinition,
    /// The braced form of `ViewBody`. `SysML` only.
    ///
    /// ```text
    /// ViewBodyItem = DefinitionBodyItem | ElementFilterMember
    ///              | ViewRenderingMember | Expose                  SysML 8.2.2.26.2
    /// ```
    ///
    /// `ViewDefinition`'s items and `Expose`, which a view usage exposes and a definition
    /// does not. A variant of its own because the item set differs in the grammar, as
    /// `Action` was before anything it decided differed.
    View,
    /// The braced form of `TypeBody`. `KerML` only — what a classifier holds.
    Type,
    /// The item run of `KerML`'s `FunctionBodyPart` (8.2.5.7.1): a `TypeBody`'s items,
    /// `TypeBodyElement`, and `ReturnFeatureMember`, then a `ResultExpressionMember`.
    /// Everything `Type` answers this answers the same way but for those two.
    Function,
}

impl Body {
    /// The membership node a nested element is owned through.
    ///
    /// In a `SysML` definition or action body the answer depends on the ELEMENT, not only
    /// on the body: `DefinitionMember` owns definitions alone (8.2.2.6.1), and a usage is
    /// owned through the membership its body's item production names for its class.
    /// Until this distinction was drawn every usage in these bodies was built as a
    /// `DefinitionMember`, a node the grammar gives no usage.
    pub(super) fn member(self, language: Language, element: MemberElement) -> SyntaxKind {
        match (self, language, element) {
            // A requirement body owns its DefinitionBodyItem alternative exactly as a
            // definition body does; what it owns differently owns itself, through
            // SubjectMember. A calculation body reaches DefinitionMember too, by the
            // other road: CalculationBodyItem to ActionBodyItem to NonBehaviorBodyItem,
            // whose third alternative it is (8.2.2.17.1).
            (
                Self::Definition
                | Self::Requirement
                | Self::ViewDefinition
                | Self::View
                | Self::Calculation
                | Self::Case
                | Self::Action
                | Self::State
                | Self::Interface,
                _,
                MemberElement::Other,
            ) => SyntaxKind::DefinitionMember,
            // InterfaceBodyItem names its own two usage members (8.2.2.14.1), and a
            // StructureUsageElement or BehaviorUsageElement is an
            // InterfaceOccurrenceUsageElement.
            (Self::Interface, _, MemberElement::Usage(UsageClass::NonOccurrence)) => {
                SyntaxKind::InterfaceNonOccurrenceUsageMember
            }
            (Self::Interface, _, MemberElement::Usage(_)) => {
                SyntaxKind::InterfaceOccurrenceUsageMember
            }
            // Both DefinitionBodyItem (8.2.2.6.1) and NonBehaviorBodyItem (8.2.2.17.1)
            // name NonOccurrenceUsageMember.
            (
                Self::Definition
                | Self::Requirement
                | Self::ViewDefinition
                | Self::View
                | Self::Calculation
                | Self::Case
                | Self::Action
                | Self::State,
                _,
                MemberElement::Usage(UsageClass::NonOccurrence),
            ) => SyntaxKind::NonOccurrenceUsageMember,
            // DefinitionBodyItem: `SourceSuccessionMember? OccurrenceUsageMember`
            // (8.2.2.6.1), and OccurrenceUsageElement is both of the other classes.
            (
                Self::Definition | Self::Requirement | Self::ViewDefinition | Self::View,
                _,
                MemberElement::Usage(_),
            ) => SyntaxKind::OccurrenceUsageMember,
            // NonBehaviorBodyItem: `SourceSuccessionMember? StructureUsageMember`
            // (8.2.2.17.1), which StateBodyItem reaches as its first alternative
            // (8.2.2.18.1).
            // A case body reaches both through CaseBodyItem's ActionBodyItem (8.2.2.22).
            (
                Self::Calculation | Self::Case | Self::Action | Self::State,
                _,
                MemberElement::Usage(UsageClass::Structure),
            ) => SyntaxKind::StructureUsageMember,
            // ActionBodyItem's third alternative: `SourceSuccessionMember?
            // ActionBehaviorMember ActionTargetSuccessionMember*`, and
            // ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember (8.2.2.17.1).
            // Only the member is read here; `source_succession_item` reads the `then`
            // before it and `behaviour_targets` the target successions after it.
            // StateBodyItem's second alternative names the same member, with target
            // transitions after it rather than target successions (8.2.2.18.1).
            (
                Self::Calculation | Self::Case | Self::Action | Self::State,
                _,
                MemberElement::Usage(UsageClass::Behavior),
            ) => SyntaxKind::BehaviorUsageMember,
            (Self::Calculation | Self::Case | Self::Action, _, MemberElement::ActionNode) => {
                SyntaxKind::ActionNodeMember
            }
            (_, Language::SysMl, _) => SyntaxKind::PackageMember,
            // Both KerML bodies own a non-feature through the same membership.
            // `TypeBodyElement` is `NonFeatureMember | FeatureMember | AliasMember |
            // Import` (8.2.4.1.1) and `NamespaceBodyElement` reaches `NonFeatureMember`
            // too (8.2.3.4.1), so `Body::Type` needs no arm of its own here. Their features
            // differ, and are dispatched before this is asked (`feature_member`).
            (_, Language::KerMl, _) => SyntaxKind::NonFeatureMember,
        }
    }

    /// Whether `ElementFilterMember` is one of this body's alternatives.
    pub(super) fn admits_filter(self, language: Language) -> bool {
        match self {
            // ViewDefinitionBodyItem and ViewBodyItem name it (SysML 8.2.2.26.1, .2).
            Self::Package | Self::ViewDefinition | Self::View => true,
            Self::Root => language == Language::SysMl,
            // TypeBodyElement has no ElementFilterMember alternative, and neither
            // DefinitionBodyItem, RequirementBodyItem nor NonBehaviorBodyItem reaches one.
            Self::Definition
            | Self::Requirement
            | Self::Calculation
            | Self::Case
            | Self::Action
            | Self::State
            | Self::Interface
            | Self::Type
            | Self::Function => false,
        }
    }

    /// Whether `SubjectMember` is one of this body's alternatives.
    ///
    /// `RequirementBodyItem` (`SysML` 8.2.2.21.1) and `CaseBodyItem` (8.2.2.22) reach it,
    /// and nothing else does.
    ///
    /// Asked separately from `member`, for the reason `admits_filter` is: what a body
    /// owns its ordinary members through and which extra alternatives it has are two
    /// questions, and deriving one from the other admits a `subject` in a definition
    /// body — which `DefinitionBodyItem` does not have.
    pub(super) fn admits_subject(self) -> bool {
        matches!(self, Self::Requirement | Self::Case)
    }

    /// Whether `ActorMember` is one of this body's alternatives.
    ///
    /// The same two item productions as `admits_subject`: `RequirementBodyItem`
    /// (`SysML` 8.2.2.21.1) and `CaseBodyItem` (8.2.2.22). Asked separately because
    /// `StakeholderMember`, the next of the six, is `RequirementBodyItem`'s alone.
    /// `validateActorMembershipOwningType` (8.3.21.2, receipt e2ea19a6) names requirement
    /// and case owners, with `oclIsKindOf`, so analysis and use cases qualify. The grammar
    /// is WIDER than that: `RequirementConstraintUsage`'s reference alternative takes a
    /// `RequirementBody` (8.2.2.21.1), so `require c { actor a; }` gives an actor to a
    /// `ConstraintUsage`. It parses, and the constraint is `sv2-resolve`'s to raise.
    pub(super) fn admits_actor(self) -> bool {
        matches!(self, Self::Requirement | Self::Case)
    }

    /// Whether `StakeholderMember` is one of this body's alternatives.
    ///
    /// `RequirementBodyItem` alone names it (`SysML` 8.2.2.21.1); `CaseBodyItem` (8.2.2.22)
    /// has `ActorMember` and not this. `validateStakeholderMembershipOwningType` (8.3.21.12,
    /// receipt 9d209633) says the same of the owner, a requirement definition or usage.
    pub(super) fn admits_stakeholder(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether `FramedConcernMember` is one of this body's alternatives.
    ///
    /// `RequirementBodyItem` alone names it (`SysML` 8.2.2.21.1), as it alone names
    /// `StakeholderMember`.
    pub(super) fn admits_framed_concern(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether `ObjectiveMember` is one of this body's alternatives.
    ///
    /// `CaseBodyItem` alone names it (`SysML` 8.2.2.22). That its owner is a case is also
    /// `validateObjectiveMembershipOwningType` (8.3.22.4), and "at most one" is
    /// `validateCaseDefinitionOnlyOneObjective` and its usage twin (8.3.22.2, 8.3.22.3):
    /// constraints, not grammar, so two objectives read and are `sv2-resolve`'s to raise.
    pub(super) fn admits_objective(self) -> bool {
        matches!(self, Self::Case)
    }

    /// Whether `VariantUsageMember` is one of this body's alternatives.
    ///
    /// It is an alternative of `DefinitionBodyItem` (`SysML` 8.2.2.6.1), which a
    /// definition or usage body and a requirement body reach (8.2.2.21.1), and of
    /// `NonBehaviorBodyItem` (8.2.2.17.1), which an action or calculation body reaches
    /// through `ActionBodyItem`, as a case body does through `CaseBodyItem` (8.2.2.22),
    /// and a state body reaches through `StateBodyItem` (8.2.2.18.1).
    /// `InterfaceBodyItem` (8.2.2.14.1) names it directly, and the two view bodies reach
    /// it through their `DefinitionBodyItem` alternative (8.2.2.26.1, .2). NOT
    /// `PackageBodyElement` (8.2.2.5.1), nor any `KerML` body: `KerML` has no variants.
    ///
    /// NOT only a variation's body, although "variant usages may only be declared within
    /// a variation" (7.6.7, receipt 5a7843af). That is
    /// `validateVariantMembershipOwningNamespace` (8.3.6.5, receipt 49805baa), a
    /// constraint on the membership's owner, which is `sv2-resolve`'s to raise; the
    /// element still enters the IR (ADR-0002).
    pub(super) fn admits_variant(self) -> bool {
        matches!(
            self,
            Self::Definition
                | Self::Requirement
                | Self::ViewDefinition
                | Self::View
                | Self::Action
                | Self::Calculation
                | Self::Case
                | Self::State
                | Self::Interface
        )
    }

    /// Whether `ViewRenderingMember` is one of this body's alternatives.
    ///
    /// `ViewDefinitionBodyItem` and `ViewBodyItem` name it (`SysML` 8.2.2.26.1, .2), and
    /// nothing else does; that its owner is a view is also
    /// `validateViewRenderingMembershipOwningType` (8.3.26.10, receipt 75857b4b). "Only
    /// one" is `validateViewDefinitionOnlyOneViewRendering` and its usage twin, constraints
    /// and not grammar, so two `render`s read.
    pub(super) fn admits_render(self) -> bool {
        matches!(self, Self::ViewDefinition | Self::View)
    }

    /// Whether `Expose` is one of this body's alternatives.
    ///
    /// `ViewBodyItem` alone names it (`SysML` 8.2.2.26.2); `ViewDefinitionBodyItem` does
    /// not, and `validateExposeOwningNamespace` (8.3.26.2) says the owner is a `ViewUsage`.
    pub(super) fn admits_expose(self) -> bool {
        matches!(self, Self::View)
    }

    /// Whether `TransitionUsageMember` is one of this body's alternatives.
    ///
    /// `StateBodyItem` alone names it (`SysML` 8.2.2.18.1). A transition "can be used within
    /// non-parallel states" (7.18.3, receipt 6e6e9493); that a parallel state holds none
    /// is `validateStateDefinitionParallelSubactions` and
    /// `validateStateUsageParallelSubactions` (8.3.18.5, 8.3.18.6), constraints on the
    /// owner and not grammar, so `parallel` bodies read the same items.
    pub(super) fn admits_transition(self) -> bool {
        matches!(self, Self::State)
    }

    /// Whether `EntryActionMember`, `DoActionMember` and `ExitActionMember` are this
    /// body's alternatives.
    ///
    /// `StateBodyItem` alone names them (`SysML` 8.2.2.18.1). A `do` inside a transition
    /// is its `EffectBehaviorMember` instead, read within the transition, so it never
    /// reaches an item position. "At most one of each" (7.18.2, receipt 42b13f63) is
    /// `validateStateDefinitionStateSubactionKind` and its usage twin, not grammar.
    pub(super) fn admits_state_action(self) -> bool {
        matches!(self, Self::State)
    }

    /// Whether `RequirementConstraintMember` is one of this body's alternatives.
    ///
    /// Asked separately from `admits_subject`, and the two disagree about `Case`:
    /// `CaseBodyItem` reaches `SubjectMember` and does NOT reach
    /// `RequirementConstraintMember` (`SysML` 8.2.2.21.1 against 8.2.2.22). A
    /// `require` belongs in a case's objective, which is a `RequirementBody`.
    pub(super) fn admits_requirement_constraint(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether `RequirementVerificationMember` is one of this body's alternatives.
    ///
    /// `RequirementBodyItem` alone names it (`SysML` 8.2.2.21.1), and a case's objective
    /// is a `RequirementBody` (8.2.2.22), which is where the corpus writes `verify`. That
    /// its owner must be an objective is `validateRequirementVerificationMembershipOwningType`
    /// (8.3.24.2, receipt 213c087b), a constraint: `verify` in a plain requirement
    /// definition parses and is `sv2-resolve`'s to refuse.
    pub(super) fn admits_requirement_verification(self) -> bool {
        matches!(self, Self::Requirement)
    }

    /// Whether this body's items may be followed by a `ResultExpressionMember`.
    ///
    /// `CalculationBodyPart = CalculationBodyItem* ResultExpressionMember?`
    /// (`SysML` 8.2.2.19), and `CaseBody` braces the same shape (8.2.2.22); no other
    /// implemented body ends in an expression. The item
    /// loop has to stop before it, because an expression is not a member and the loop
    /// would otherwise recover over it one token at a time.
    pub(super) fn ends_in_result_expression(self) -> bool {
        matches!(self, Self::Calculation | Self::Case | Self::Function)
    }

    /// Whether `ReturnParameterMember` is one of this body's alternatives.
    ///
    /// `CalculationBodyItem = ActionBodyItem | ReturnParameterMember`
    /// (`SysML` 8.2.2.19), and it is the second alternative — so the containment runs
    /// from calculation to action and NOT the other way. `Body::Action` and
    /// `Body::Calculation` share the item loop, which is exactly how a `return` could
    /// come to be admitted in an action body by accident;
    /// tests/rejection/return-parameter-member-is-not-an-action-body-item.sysml is the
    /// file that fails if it ever is.
    ///
    /// `Case` by deviation `CaseBodyItem` (`follow_xtext`): 8.2.2.22 leaves
    /// `ReturnParameterMember` out of `CaseBodyItem`, while 7.22.2 declares a case "a kind
    /// of calculation definition or usage" (receipt eb25a69f) and 7.23.2's own example
    /// writes `return` in an analysis case body (receipt 2aa2d6ce). `body_specific_item`
    /// notes the departure where it reads one.
    pub(super) fn admits_return_parameter(self) -> bool {
        matches!(self, Self::Calculation | Self::Case)
    }

    /// Whether `ActionBodyItem`'s alternatives beyond `NonBehaviorBodyItem` belong to
    /// this body — today, `InitialNodeMember`.
    ///
    /// `ActionBodyItem` is reached by eight productions: `ActionBody` (`SysML`
    /// 8.2.2.17.1), `CalculationBodyItem` (8.2.2.19), `CaseBodyItem` (8.2.2.22),
    /// `ActionBodyParameter` (8.2.2.17.7), and the braced bodies of the four
    /// `Transition*ActionUsage`s (8.2.2.18.3). NOT by `RequirementBodyItem`, which reaches
    /// only `DefinitionBodyItem` (8.2.2.21.1), and NOT by `DefinitionBodyItem` (8.2.2.6.1).
    /// A comment on `admits_return_parameter` once said the opposite; it was false, and
    /// widening this to `Requirement` on its word would have admitted `first` where the
    /// grammar has none. tests/rejection/initial-node-member-is-not-a-requirement-body-item.sysml
    /// fails if it ever is. Of the eight, `ActionBody`, `CalculationBody` and `CaseBody`
    /// have a `Body` variant — the rest are unimplemented — so `Action`, `Calculation` and
    /// `Case` are the whole of it today, and each of the others joins this when its body
    /// lands.
    ///
    /// `Calculation` covers `ConstraintDefinition` too, which shares `CalculationBody`
    /// (8.2.2.20): a constraint body admits `first` by the grammar, however unusual.
    pub(super) fn admits_action_body_item(self) -> bool {
        matches!(self, Self::Action | Self::Calculation | Self::Case)
    }

    /// Whether `SourceSuccessionMember` may prefix a member here.
    ///
    /// `DefinitionBodyItem` puts it before `OccurrenceUsageMember` (8.2.2.6.1), and a
    /// requirement body reaches `DefinitionBodyItem` (8.2.2.21.1); `NonBehaviorBodyItem`
    /// puts it before `StructureUsageMember` and `ActionBodyItem` before
    /// `ActionBehaviorMember` (8.2.2.17.1), both reached from action and calculation
    /// bodies; `StateBodyItem` puts it before `BehaviorUsageMember` and reaches
    /// `NonBehaviorBodyItem` (8.2.2.18.1); `InterfaceBodyItem` puts it before
    /// `InterfaceOccurrenceUsageMember` (8.2.2.14.1). `PackageBodyElement` (8.2.2.5.1) has
    /// no such alternative, nor has `KerML`.
    ///
    /// NOT only action bodies, although 7.17.4 says its shorthands "may be used only
    /// within the body of an action definition or usage" (receipt 339ef468). That
    /// sentence scopes the ACTION shorthands of that clause; `DefinitionBodyItem` states
    /// `then` before any occurrence usage unconditionally, and the corpus uses it so:
    /// `then snapshot part vehicle_1_t1 {` inside an `individual part` (training/28.
    /// Individuals/Individuals and Roles-1.sysml:18), and `then event occurrence …` in
    /// the Occurrences examples of training/27.
    pub(super) fn admits_source_succession(self) -> bool {
        matches!(
            self,
            Self::Definition
                | Self::Requirement
                | Self::ViewDefinition
                | Self::View
                | Self::Action
                | Self::Calculation
                | Self::Case
                | Self::State
                | Self::Interface
        )
    }
}

impl Parser<'_> {
    /// `';' | '{' <body>'s items '}'` as a `node`, the shape every `SysML` body shares.
    pub(super) fn braced_body(&mut self, node: SyntaxKind, body: Body, what: &str) {
        self.eat_trivia();
        self.start_node(node);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), body);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected(what);
        }
        self.finish_node();
    }
}
