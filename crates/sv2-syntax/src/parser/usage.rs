// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Usages, `SysML` 8.2.2.6: usage prefixes, the usage element dispatch, reference usages,
//! variants, event occurrences, and the usage declaration.

use crate::counter::{Counter, count};
use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::lookahead::keyword;

/// A usage production whose whole rule is `<prefix> KEYWORD Usage`.
#[derive(Clone, Copy)]
pub(super) struct SimpleUsage {
    /// The one keyword that says which production this is.
    keyword: &'static str,
    /// The node the production builds.
    node: SyntaxKind,
    /// Whether the prefix is an `OccurrenceUsagePrefix` rather than a `UsagePrefix`.
    ///
    /// An occurrence may carry `'individual'` and a `PortionKind` (`SysML` 8.2.2.9.2);
    /// an attribute and an enumeration may not, because neither is an occurrence.
    is_occurrence: bool,
    /// Which of 8.2.2.6.4's three sorts of usage this is, which decides the membership
    /// a body owns it through. Stated per usage rather than read off `is_occurrence`:
    /// the two agree for these seven, but one is about the prefix and the other about
    /// the element, and `ActionUsage` is an occurrence whose class is `Behavior`.
    pub(super) class: UsageClass,
}

/// The three sorts of usage `SysML` 8.2.2.6.4 names, each with its own element
/// alternation and so its own membership.
///
/// ```text
/// NonOccurrenceUsageElement = DefaultReferenceUsage | ReferenceUsage | AttributeUsage
///                           | EnumerationUsage | BindingConnectorAsUsage
///                           | SuccessionAsUsage | ExtendedUsage
/// StructureUsageElement     = OccurrenceUsage | IndividualUsage | PortionUsage
///                           | EventOccurrenceUsage | ItemUsage | PartUsage | ViewUsage
///                           | RenderingUsage | PortUsage | ConnectionUsage | ...
/// BehaviorUsageElement      = ActionUsage | CalculationUsage | StateUsage | ...
///                           | PerformActionUsage | ...
/// ```
///
/// `OccurrenceUsageElement = StructureUsageElement | BehaviorUsageElement`, so a
/// definition body, which asks only occurrence or not, does not tell the last two apart
/// and an action body does.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum UsageClass {
    NonOccurrence,
    Structure,
    Behavior,
}

/// Every usage production that is a prefix, one keyword and the `Usage` spine.
///
/// Ordered as the clauses number them. The keywords are disjoint, so the order does
/// not decide anything — `at_simple_usage` takes the one whose keyword is written.
pub(super) const SIMPLE_USAGES: [SimpleUsage; 7] = [
    SimpleUsage {
        keyword: "attribute",
        node: SyntaxKind::AttributeUsage,
        is_occurrence: false,
        class: UsageClass::NonOccurrence,
    },
    SimpleUsage {
        keyword: "enum",
        node: SyntaxKind::EnumerationUsage,
        is_occurrence: false,
        class: UsageClass::NonOccurrence,
    },
    SimpleUsage {
        keyword: "occurrence",
        node: SyntaxKind::OccurrenceUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "item",
        node: SyntaxKind::ItemUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "part",
        node: SyntaxKind::PartUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "port",
        node: SyntaxKind::PortUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
    SimpleUsage {
        keyword: "rendering",
        node: SyntaxKind::RenderingUsage,
        is_occurrence: true,
        class: UsageClass::Structure,
    },
];

impl Parser<'_> {
    /// The index just past a `BasicUsagePrefix` written from the `n`th token.
    ///
    /// `BasicUsagePrefix = RefPrefix 'ref'?` over `RefPrefix = FeatureDirection?
    /// 'derived'? ( 'abstract' | 'variation' )? 'constant'?` (`SysML` 8.2.2.6.2).
    /// Every part is optional, so this returns `n` unchanged when none is written.
    ///
    /// The keywords are counted in the clause's order and each at most once, which is
    /// what makes `abstract in part p;` two errors rather than a longer prefix.
    ///
    /// NOT a whole `UsagePrefix`: `UsagePrefix = UnextendedUsagePrefix
    /// UsageExtensionKeyword*` and `UnextendedUsagePrefix = EndUsagePrefix |
    /// BasicUsagePrefix`, and `skip_usage_prefix` is the one that also looks past an
    /// `EndUsagePrefix` and the `UsageExtensionKeyword`s after either.
    fn skip_basic_usage_prefix(&self, n: usize) -> usize {
        count(Counter::SkipBasicUsagePrefix);
        // Asked many times from one start, like `skip_prefix_metadata` (roadmap Phase 4).
        self.memoized(
            &self.basic_usage_prefix_ends,
            n,
            Self::compute_basic_usage_prefix,
        )
    }

    /// `skip_basic_usage_prefix`'s walk, when its cache does not have the answer.
    fn compute_basic_usage_prefix(&self, n: usize) -> usize {
        count(Counter::SkipBasicUsagePrefixComputed);
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract", "variation"],
            &["constant"],
            &["ref"],
        ] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
    }

    /// The index just past an `OccurrenceUsagePrefix` written from the `n`th token.
    ///
    /// `OccurrenceUsagePrefix = ( EndUsagePrefix | BasicUsagePrefix 'individual'?
    /// PortionKind? ) UsageExtensionKeyword*` (`SysML` 8.2.2.9.2) — a
    /// `BasicUsagePrefix` and the two keywords only an occurrence may carry.
    pub(super) fn skip_occurrence_usage_prefix(&self, n: usize) -> usize {
        count(Counter::SkipOccurrenceUsagePrefix);
        // Asked many times from one start, like `skip_prefix_metadata` (roadmap Phase 4).
        self.memoized(
            &self.occurrence_usage_prefix_ends,
            n,
            Self::compute_occurrence_usage_prefix,
        )
    }

    /// `skip_occurrence_usage_prefix`'s walk, when its cache does not have the answer.
    fn compute_occurrence_usage_prefix(&self, n: usize) -> usize {
        count(Counter::SkipOccurrenceUsagePrefixComputed);
        // EndUsagePrefix, the first alternative by deviation OccurrenceUsagePrefix
        // (follow_xtext), excludes the rest: an end is not also individual.
        if let Some(kind) = self.skip_end_usage_prefix(n) {
            return self.skip_prefix_metadata(kind);
        }
        let mut n = self.skip_basic_usage_prefix(n);
        for words in [&["individual"][..], &["snapshot", "timeslice"]] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        self.skip_prefix_metadata(n)
    }

    /// The index just past a `UsagePrefix` written from the `n`th token:
    /// `UnextendedUsagePrefix UsageExtensionKeyword*`, with `UnextendedUsagePrefix =
    /// EndUsagePrefix | BasicUsagePrefix` (`SysML` 8.2.2.6.2).
    pub(super) fn skip_usage_prefix(&self, n: usize) -> usize {
        self.skip_prefix_metadata(self.skip_unextended_usage_prefix(n))
    }

    /// The index just past an `UnextendedUsagePrefix` written from the `n`th token.
    fn skip_unextended_usage_prefix(&self, n: usize) -> usize {
        self.skip_end_usage_prefix(n)
            .unwrap_or_else(|| self.skip_basic_usage_prefix(n))
    }

    /// The index of the usage's kind keyword after an `EndUsagePrefix` at the `n`th token,
    /// or `None` when no `end` is there or no kind keyword follows it.
    ///
    /// `EndUsagePrefix = 'end' OwnedCrossFeatureMember?` (`SysML` 8.2.2.6.2), and an owned
    /// cross feature is "declared between the end and kind keywords" (7.13.2, receipt
    /// 5a3a8867): everything from the `end` to the first kind keyword is the cross
    /// feature, a `BasicUsagePrefix UsageDeclaration`, which writes no `;`, brace or `=`.
    /// With no kind keyword before one of those, the `end` is `DefaultReferenceUsage`'s
    /// bare one (deviation `DefaultReferenceUsage`), which owns no cross feature.
    pub(super) fn skip_end_usage_prefix(&self, n: usize) -> Option<usize> {
        if !self.nth_is_keyword(n, "end") {
            return None;
        }
        let mut k = n + 1;
        loop {
            let token = self.peek_nth(k)?;
            if matches!(
                token.kind,
                SyntaxKind::Semicolon | SyntaxKind::LBrace | SyntaxKind::RBrace | SyntaxKind::Eq
            ) {
                return None;
            }
            if self.at_end_kind(k) {
                return Some(k);
            }
            k += 1;
        }
    }

    /// Whether the `k`th token is the kind keyword an `EndUsagePrefix` is followed by.
    ///
    /// A keyword a member opens on, not counting the prefix keywords a cross feature's
    /// own `BasicUsagePrefix` may write. `ref` is both: it is `ReferenceUsage`'s kind
    /// keyword unless a kind keyword stands LATER in the same end declaration, because
    /// that production's `Usage` is a declaration and a completion, and a declaration
    /// never contains a kind keyword. So in `end ref part p;` and in `end ref x : T part
    /// p;` the `ref` opens the cross feature and `part` is the kind, while in `end ref x;`
    /// the `ref` is the kind.
    pub(super) fn at_end_kind(&self, k: usize) -> bool {
        if self.nth_is_keyword(k, "ref") {
            return !self.kind_follows(k + 1);
        }
        let prefix = self.skip_basic_usage_prefix(k) != k
            || ["individual", "snapshot", "timeslice"]
                .iter()
                .any(|word| self.nth_is_keyword(k, word));
        !prefix && self.at_sysml_keyword_member(k)
    }

    // production: EndUsagePrefix@sysml
    //
    // EndUsagePrefix : Usage =
    //     isEnd ?= 'end' ( ownedRelationship += OwnedCrossFeatureMember )?
    //                                                                (SysML 8.2.2.6.2)
    //
    // "End features are declared as usages (see 7.6.3), prefixed by the keyword end"
    // (7.13.2, receipt 5a3a8867). The cross feature is present when anything stands
    // between `end` and the kind keyword; see `skip_end_usage_prefix`. Its BasicUsagePrefix
    // node is built even when empty, as every prefix node here is.
    fn end_usage_prefix(&mut self) {
        let cross = self.skip_end_usage_prefix(0).is_some_and(|kind| kind > 1);
        self.eat_trivia();
        self.start_node(SyntaxKind::EndUsagePrefix);
        self.expect_keyword("end");
        if cross {
            self.owned_cross_feature_member();
        }
        self.finish_node();
    }

    /// Whether an `ExtendedUsage` starts at the `n`th meaningful token.
    ///
    /// `UnextendedUsagePrefix UsageExtensionKeyword+ Usage` (`SysML` 8.2.2.27): at least
    /// one `#`, and then what a `Usage` opens on, which is a `UsageDeclaration` or, with
    /// no declaration, a `UsageCompletion` (8.2.2.6.2) -- a name, a short name's `<`, a
    /// feature specialization or a multiplicity; a `ValuePart`'s `=`, `:=` or `default`;
    /// a `UsageBody`'s `;` or `{`. A kind keyword there makes the `#`s that usage's
    /// prefix instead, and `def` makes them an `ExtendedDefinition`'s.
    pub(super) fn at_extended_usage(&self, n: usize) -> bool {
        let k = self.skip_unextended_usage_prefix(n);
        let after = self.skip_prefix_metadata(k);
        after > k && self.nth_opens_usage(after)
    }

    /// Whether a `Usage` opens at the `n`th meaningful token.
    ///
    /// `Usage = UsageDeclaration UsageCompletion` (`SysML` 8.2.2.6.2), every part of it
    /// optional but the `UsageBody`: a name, a short name's `<`, a feature specialization
    /// or a multiplicity; a `ValuePart`'s `=`, `:=` or `default`; a `UsageBody`'s `;` or
    /// `{`. A reserved keyword opens none of them (8.2.2.1.2), which is what tells a usage
    /// with no kind keyword from the keyword usage its prefix would otherwise belong to.
    fn nth_opens_usage(&self, n: usize) -> bool {
        self.nth_is_name(n)
            || self.nth_is(n, SyntaxKind::Lt)
            || self.nth_at_feature_specialization(n)
            || self.nth_is(n, SyntaxKind::LBracket)
            || self.nth_is(n, SyntaxKind::Eq)
            || self.nth_is(n, SyntaxKind::ColonEq)
            || self.nth_is_keyword(n, "default")
            || self.nth_is(n, SyntaxKind::Semicolon)
            || self.nth_is(n, SyntaxKind::LBrace)
    }

    // production: ExtendedUsage@sysml
    //
    // ExtendedUsage : Usage =
    //     UnextendedUsagePrefix UsageExtensionKeyword+ Usage       (SysML 8.2.2.27)
    //
    // `#situation batteryLow;` (7.27.4, receipt 0f2c5bd1): a usage declared with no
    // language-defined keyword. A NonOccurrenceUsageElement (8.2.2.6.4). The prefix is
    // written inline, so there is no UsagePrefix node, and it is UNEXTENDED: the `#`s
    // after it are this production's own `+`.
    //
    // implied specialization: the baseType of each SemanticMetadata keyword (7.27.3).
    fn extended_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExtendedUsage);
        self.unextended_usage_prefix();
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.usage();
        self.finish_node();
    }

    // production: VariantUsageMember@sysml
    //
    // VariantUsageMember : VariantMembership =
    //     MemberPrefix 'variant'
    //     ownedVariantUsage = VariantUsageElement                  (SysML 8.2.2.6.1)
    //
    // A VariantMembership, which is an OwningMembership and NOT a FeatureMembership, so a
    // variant is an ownedMember but not an ownedFeature of its owner (8.4.2.3, receipt
    // 4ad35baf) — which is why it has a node of its own rather than the body's usage
    // member. VariantUsageElement is marked at `variant_usage_element`. The element takes no MemberPrefix of its own, and the usage inside
    // keeps its own prefix: `variant part p;`, `variant attribute a = 70[mm];`.
    //
    // constraint: VariantMembership::validateVariantMembershipOwningNamespace (8.3.6.5,
    //     receipt 49805baa) — the owner must be a variation. sv2-resolve's, not the
    //     parser's (ADR-0002); `Body::admits_variant` says why.
    pub(super) fn variant_usage_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::VariantUsageMember);
        self.member_prefix();
        self.expect_keyword("variant");
        self.variant_usage_element();
        self.finish_node();
    }

    // production: VariantUsageElement@sysml
    //
    // VariantUsageElement : Usage =
    //       VariantReference | ReferenceUsage | AttributeUsage | BindingConnectorAsUsage
    //     | SuccessionAsUsage | OccurrenceUsage | IndividualUsage | PortionUsage
    //     | EventOccurrenceUsage | ItemUsage | PartUsage | ViewUsage | RenderingUsage
    //     | PortUsage | ConnectionUsage | InterfaceUsage | AllocationUsage | Message
    //     | FlowUsage | SuccessionFlowUsage | BehaviorUsageElement   (SysML 8.2.2.6.4)
    //
    // All twenty-one alternatives: VariantReference on a NAME, and the rest through
    // `usage_element_of_class` with the three exclusions below refused first.
    //
    // It is UsageElement less three of NonOccurrenceUsageElement's alternatives (8.2.2.6.4):
    // DefaultReferenceUsage, replaced by VariantReference; EnumerationUsage; and
    // ExtendedUsage. The exclusions are the grammar's, stated by the two alternations and
    // nothing else. 8.4.2.3 (receipt 4ad35baf) excludes enumerations from being VARIATIONS,
    // which is consistent but is not this rule. A NAME is where the two usage alternations
    // part: VariantReference opens on one, and no keyword usage does.
    //
    // ExtendedUsage (`UnextendedUsagePrefix UsageExtensionKeyword+ Usage`, 8.2.2.6.4) is
    // read by `usage_element_of_class`, so it is refused here as EnumerationUsage is.
    fn variant_usage_element(&mut self) {
        if self.at_name() {
            self.variant_reference();
        } else if self.at_default_reference_usage(0)
            || self.at_extended_usage(0)
            || self
                .at_simple_usage(0)
                .is_some_and(|usage| usage.node == SyntaxKind::EnumerationUsage)
        {
            // Read by `usage_element_of_class`, and no alternative of this production.
            // Nothing is consumed: the body loop reads what follows as the member it is,
            // so the one diagnostic is the `variant` before it.
            self.error_expected("a variant usage or the name of a usage after `variant`");
        } else if self.usage_element_of_class().is_none() {
            self.error_expected("a variant usage or the name of a usage after `variant`");
        }
    }

    // production: VariantReference@sysml
    //
    // VariantReference : ReferenceUsage =
    //     ownedRelationship += OwnedReferenceSubsetting
    //     FeatureSpecialization* UsageBody                         (SysML 8.2.2.6.3)
    //
    // "A non-variant usage can also be declared to act as a variant of a variation by not
    // including a kind keyword in the variant declaration and, instead, following the
    // variant keyword with the identification of a separately declared usage" (7.6.7,
    // receipt 5a7843af). NOT a DefaultReferenceUsage: it declares no name, its first part
    // is the reference to the usage it varies, and it has no ValuePart.
    //
    // 7.6.7 goes on to say such a declaration "may also optionally further constrain the
    // variant usage by including a multiplicity", and the production writes no
    // MultiplicityPart. The grammar is followed; `variant x[1];` is reported.
    fn variant_reference(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::VariantReference);
        self.owned_reference_subsetting();
        while self.at_feature_specialization() {
            self.feature_specialization();
        }
        self.usage_body();
        self.finish_node();
    }

    /// `SysML`'s `UsageElement`. Returns whether one was read.
    ///
    /// `UsageElement` is stated at `SysML` 8.2.2.5.2 beside `DefinitionElement`. It is
    /// reachable from `PackageMember` (8.2.2.5.1) and not from `NamespaceMember`
    /// (`KerML` 8.2.3.4.1), which is why the caller asks this only for `SysML` — `KerML`
    /// has no usages at all. The three clauses are named separately because they are
    /// three different facts; an earlier revision cited 8.2.2.6.1 for all of it.
    ///
    /// ORDER IS LOAD-BEARING, and each step of it is a defect that was fixed once:
    ///
    /// - the keyword usages come before the two reference usages, because
    ///   `at_reference_usage` sees the `ref` in `ref attribute y;` too, and that `ref`
    ///   is an `AttributeUsage`'s `BasicUsagePrefix` rather than a `ReferenceUsage`;
    /// - `ExtendedUsage` comes after the keyword usages, because their prefixes look past
    ///   the same `#`s and a kind keyword after them makes those the usage's prefix
    ///   (`SysML` 8.2.2.6.2), and it needs at least one `#`, which is what separates it
    ///   from `DefaultReferenceUsage`;
    /// - `DefaultReferenceUsage` is last of all, because it is the usage with no
    ///   keyword, so everything that opens with one has already been taken.
    pub(super) fn usage_element(&mut self) -> bool {
        self.usage_element_of_class().is_some()
    }

    // production: UsageElement@sysml
    //
    // UsageElement : Usage =
    //     NonOccurrenceUsageElement | OccurrenceUsageElement      (SysML 8.2.2.5.2)
    //
    // `usage_element`, answering which of 8.2.2.6.4's classes the usage read is in. Both
    // alternatives, the occurrence usages first: the order constraints above all hold
    // within `non_occurrence_usage_element`, whose ReferenceUsage, ExtendedUsage and
    // DefaultReferenceUsage are asked after every keyword usage of both classes.
    pub(super) fn usage_element_of_class(&mut self) -> Option<UsageClass> {
        self.occurrence_usage_element().or_else(|| {
            self.non_occurrence_usage_element()
                .then_some(UsageClass::NonOccurrence)
        })
    }

    // production: OccurrenceUsageElement@sysml
    //
    // OccurrenceUsageElement : Usage =
    //     StructureUsageElement | BehaviorUsageElement           (SysML 8.2.2.6.4)
    //
    // Both, returning which was read.
    fn occurrence_usage_element(&mut self) -> Option<UsageClass> {
        if self.behavior_usage_element() {
            Some(UsageClass::Behavior)
        } else if self.structure_usage_element() {
            Some(UsageClass::Structure)
        } else {
            None
        }
    }

    // production: NonOccurrenceUsageElement@sysml
    //
    // NonOccurrenceUsageElement : Usage =
    //       DefaultReferenceUsage | ReferenceUsage | AttributeUsage | EnumerationUsage
    //     | BindingConnectorAsUsage | SuccessionAsUsage | ExtendedUsage
    //                                                            (SysML 8.2.2.6.4)
    //
    // All seven, returning whether one was read. The keyword usages first, then the
    // three that must follow every keyword usage (see `usage_element`): ReferenceUsage,
    // whose `ref` is a keyword usage's BasicUsagePrefix too; ExtendedUsage, which needs a
    // `#` and no kind keyword after it; and DefaultReferenceUsage, which has no keyword.
    fn non_occurrence_usage_element(&mut self) -> bool {
        if self.at_succession_as_usage(0) {
            // "a succession is not a kind of occurrence usage" (7.13.5, receipt 2abd302c).
            self.succession_as_usage();
        } else if self.at_binding_connector_as_usage(0) {
            // "a binding is not a kind of occurrence usage" (7.13.3, receipt 6db87b41).
            self.binding_connector_as_usage();
        } else if let Some(usage) = self
            .at_simple_usage(0)
            .filter(|usage| usage.class == UsageClass::NonOccurrence)
        {
            // AttributeUsage and EnumerationUsage.
            self.simple_usage(usage);
        } else if self.at_reference_usage(0) {
            self.reference_usage();
        } else if self.at_extended_usage(0) {
            self.extended_usage();
        } else if self.at_default_reference_usage(0) {
            self.default_reference_usage();
        } else {
            return false;
        }
        true
    }

    // production: StructureUsageElement@sysml
    //
    // StructureUsageElement : Usage =
    //       OccurrenceUsage | IndividualUsage | PortionUsage | EventOccurrenceUsage
    //     | ItemUsage | PartUsage | ViewUsage | RenderingUsage | PortUsage
    //     | ConnectionUsage | InterfaceUsage | AllocationUsage | Message | FlowUsage
    //     | SuccessionFlowUsage                                   (SysML 8.2.2.6.4)
    //
    // All fifteen, returning whether one was read: five from SIMPLE_USAGES, then
    // IndividualUsage and PortionUsage together, then eight by keyword. The order within
    // decides nothing, because each recogniser requires its own kind keyword or, for
    // IndividualUsage and PortionUsage, a Usage opening with none (see
    // `at_individual_or_portion_usage`). IndividualUsage and PortionUsage are asked before
    // `non_occurrence_usage_element`'s ReferenceUsage, which sees the `ref` in `ref
    // individual x;` too, and that `ref` is their BasicUsagePrefix.
    fn structure_usage_element(&mut self) -> bool {
        if let Some(usage) = self
            .at_simple_usage(0)
            .filter(|usage| usage.class == UsageClass::Structure)
        {
            // OccurrenceUsage, ItemUsage, PartUsage, PortUsage and RenderingUsage.
            self.simple_usage(usage);
        } else if let Some(node) = self.at_individual_or_portion_usage(0) {
            self.individual_or_portion_usage(node);
        } else if self.at_flow_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), though its metaclass is an ActionUsage.
            self.flow_usage();
        } else if self.at_succession_flow_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as FlowUsage is.
            self.succession_flow_usage();
        } else if self.at_message(0) {
            // A StructureUsageElement (8.2.2.6.4), as FlowUsage is.
            self.message();
        } else if self.at_connection_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as FlowUsage is.
            self.connection_usage();
        } else if self.at_interface_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as ConnectionUsage is.
            self.interface_usage();
        } else if self.at_allocation_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as ConnectionUsage is.
            self.allocation_usage();
        } else if self.at_view_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as PartUsage is.
            self.view_usage();
        } else if self.at_event_occurrence_usage(0) {
            // A StructureUsageElement (8.2.2.6.4), as OccurrenceUsage is.
            self.event_occurrence_usage();
        } else {
            return false;
        }
        true
    }

    // production: BehaviorUsageElement@sysml
    //
    // BehaviorUsageElement : Usage =
    //       ActionUsage | CalculationUsage | StateUsage | ConstraintUsage
    //     | RequirementUsage | ConcernUsage | CaseUsage | AnalysisCaseUsage
    //     | VerificationCaseUsage | UseCaseUsage | ViewpointUsage | PerformActionUsage
    //     | ExhibitStateUsage | IncludeUseCaseUsage | AssertConstraintUsage
    //     | SatisfyRequirementUsage                               (SysML 8.2.2.6.4)
    //
    // All sixteen, returning whether one was read; the four cases through CASES.
    fn behavior_usage_element(&mut self) -> bool {
        if self.at_perform_action_usage(0) {
            self.perform_action_usage();
            true
        } else if self.at_action_usage(0) {
            self.action_usage();
            true
        } else if self.at_state_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ActionUsage is.
            self.state_usage();
            true
        } else if self.at_exhibit_state_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as PerformActionUsage is.
            self.exhibit_state_usage();
            true
        } else if self.at_calculation_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ActionUsage is.
            self.calculation_usage();
            true
        } else if let Some(case) = self.at_case_usage(0) {
            // CaseUsage, AnalysisCaseUsage and UseCaseUsage are BehaviorUsageElements
            // (8.2.2.6.4).
            self.case_usage(case);
            true
        } else if self.at_include_use_case_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as PerformActionUsage is.
            self.include_use_case_usage();
            true
        } else if self.at_requirement_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ConstraintUsage is.
            self.requirement_usage();
            true
        } else if self.at_concern_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as RequirementUsage is.
            self.concern_usage();
            true
        } else if self.at_viewpoint_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ConcernUsage is.
            self.viewpoint_usage();
            true
        } else if self.at_constraint_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as AssertConstraintUsage is.
            self.constraint_usage();
            true
        } else if self.at_assert_constraint_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as ActionUsage is.
            self.assert_constraint_usage();
            true
        } else if self.at_satisfy_requirement_usage(0) {
            // A BehaviorUsageElement (8.2.2.6.4), as AssertConstraintUsage is.
            self.satisfy_requirement_usage();
            true
        } else {
            false
        }
    }

    // production: ReferenceUsage@sysml
    //
    // ReferenceUsage : ReferenceUsage =
    //     ( EndUsagePrefix | RefPrefix ) 'ref' Usage             (SysML 8.2.2.6.3)
    //
    // Both alternatives are read: `end [*] ref cause: Situation;` (validation/14-Language
    // Extensions/14c-Language Extensions.sysml:39) takes the first.
    //
    // The `ref` here is the production's own keyword, not BasicUsagePrefix's optional
    // one. `ref attribute y;` is an AttributeUsage whose prefix carries `ref`, and
    // `ref y;` is a ReferenceUsage; the difference is whether a usage keyword follows,
    // which is why `at_simple_usage` is asked first.
    fn reference_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ReferenceUsage);
        if self.at_keyword("end") {
            self.end_usage_prefix();
        } else {
            self.ref_prefix();
        }
        self.expect_keyword("ref");
        self.usage();
        self.finish_node();
    }

    /// Whether a `ReferenceUsage` starts at the `n`th meaningful token.
    ///
    /// Not when a `#` follows the `ref`: `ReferenceUsage = ( EndUsagePrefix | RefPrefix )
    /// 'ref' Usage` takes no extension keyword, and a `Usage` does not open on one, so in
    /// `ref #X y;` the `ref` is `BasicUsagePrefix`'s and the whole is an `ExtendedUsage`
    /// (`SysML` 8.2.2.6.2, 8.2.2.27).
    pub(super) fn at_reference_usage(&self, n: usize) -> bool {
        let after = self
            .skip_end_usage_prefix(n)
            .unwrap_or_else(|| self.skip_ref_prefix(n));
        self.nth_is_keyword(after, "ref") && !self.nth_is(after + 1, SyntaxKind::Hash)
    }

    /// The index just past a `RefPrefix` written from the `n`th token.
    ///
    /// `RefPrefix = FeatureDirection? 'derived'? ( 'abstract' | 'variation' )?
    /// 'constant'?` (`SysML` 8.2.2.6.2) — every part optional, and NOT including the
    /// `ref` that `BasicUsagePrefix` adds after it.
    pub(super) fn skip_ref_prefix(&self, n: usize) -> usize {
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract", "variation"],
            &["constant"],
        ] {
            if words.iter().any(|word| self.nth_is_keyword(n, word)) {
                n += 1;
            }
        }
        n
    }

    // production: DefaultReferenceUsage
    //
    // DefaultReferenceUsage : ReferenceUsage =
    //     ( isEnd ?= 'end' )? RefPrefix
    //     ( Identification FeatureSpecializationPart? | FeatureSpecializationPart )
    //     UsageCompletion                                        (SysML 8.2.2.6.2)
    //
    // A usage with NO keyword, carried by its declaration alone — SysML's analogue of
    // KerML's keywordless Feature, and the same shape: a name with an optional
    // specialization, or a bare specialization with no name. The second form is what
    // `:>> length = 4800 [mm];` is, a redefinition that names nothing.
    //
    // Unlike KerML's Feature, the bare-specialization form IS read here, because this
    // production states it directly rather than reaching it through a FeatureDeclaration
    // shared with a keyword form.
    fn default_reference_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefaultReferenceUsage);
        if self.at_keyword("end") {
            // deviation: DefaultReferenceUsage
            self.note_deviation("DefaultReferenceUsage", "`end` on a keywordless usage");
        }
        self.eat_optional_keyword("end");
        self.ref_prefix();
        if self.at_name() || self.at(SyntaxKind::Lt) {
            self.identification();
            self.optional_feature_specialization_part();
        } else {
            self.feature_specialization_part();
        }
        self.usage_completion();
        self.finish_node();
    }

    /// Whether a `DefaultReferenceUsage` starts at the `n`th meaningful token.
    ///
    /// Asked last of the usages, because it is the one with no keyword: anything that
    /// opens with `part`, `attribute`, `ref` or a definition keyword has already been
    /// taken by then, and a reserved keyword is not a name (`SysML` 8.2.2.1.2), so
    /// `package P;` is not read as a usage called `package`.
    pub(super) fn at_default_reference_usage(&self, n: usize) -> bool {
        let after = self.skip_ref_prefix(n + usize::from(self.nth_is_keyword(n, "end")));
        self.nth_is_name(after)
            || self
                .peek_nth(after)
                .is_some_and(|t| t.kind == SyntaxKind::Lt)
            || self.nth_at_feature_specialization(after)
    }

    // production: AttributeUsage
    // production: EnumerationUsage
    // production: ItemUsage
    // production: OccurrenceUsage
    // production: PartUsage
    // production: PortUsage
    // production: RenderingUsage
    //
    // AttributeUsage   = UsagePrefix           'attribute'  Usage  (SysML 8.2.2.7)
    // EnumerationUsage = UsagePrefix           'enum'       Usage  (SysML 8.2.2.8)
    // ItemUsage        = OccurrenceUsagePrefix 'item'       Usage  (SysML 8.2.2.10)
    // OccurrenceUsage  = OccurrenceUsagePrefix 'occurrence' Usage  (SysML 8.2.2.9.2)
    // PartUsage        = OccurrenceUsagePrefix 'part'       Usage  (SysML 8.2.2.11)
    // PortUsage        = OccurrenceUsagePrefix 'port'       Usage  (SysML 8.2.2.12)
    // RenderingUsage   = OccurrenceUsagePrefix 'rendering'  Usage  (SysML 8.2.2.26.3)
    //
    // Seven productions, one method, as PackageMember and DefinitionMember share
    // `membership`. They differ in exactly two things — the keyword, and whether the
    // prefix is an OccurrenceUsagePrefix or a UsagePrefix — so SIMPLE_USAGES carries
    // those two and nothing else. Writing seven near-identical methods would not make
    // any of them more faithful to its clause; it would make a difference between
    // them harder to see.
    //
    // Each is marked separately because each IS fully implemented. What none of them
    // implements lives below, in the prefixes and in FeatureSpecializationPart, and
    // is recorded there.
    fn simple_usage(&mut self, usage: SimpleUsage) {
        self.eat_trivia();
        self.start_node(usage.node);
        if usage.is_occurrence {
            self.occurrence_usage_prefix();
        } else {
            self.usage_prefix();
        }
        self.expect_keyword(usage.keyword);
        self.usage();
        self.finish_node();
    }

    pub(super) fn at_simple_usage(&self, n: usize) -> Option<SimpleUsage> {
        SIMPLE_USAGES.iter().copied().find(|usage| {
            let after = if usage.is_occurrence {
                self.skip_occurrence_usage_prefix(n)
            } else {
                self.skip_usage_prefix(n)
            };
            self.nth_is_keyword(after, usage.keyword) && !self.nth_is_keyword(after + 1, "def")
        })
    }

    // production: EventOccurrenceUsage@sysml
    //
    // EventOccurrenceUsage : EventOccurrenceUsage =
    //     OccurrenceUsagePrefix 'event'
    //     ( ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart?
    //     | 'occurrence' UsageDeclaration? )
    //     UsageCompletion                                        (SysML 8.2.2.9.2)
    //
    // "An event occurrence usage is declared like an occurrence usage ... but using the
    // kind keyword event occurrence instead of just occurrence ... [or] using just the
    // keyword event. In this case, the declaration does not include either a name or a
    // short name. Instead, the referenced event occurrence ... is identified by giving a
    // qualified name or feature chain immediately after the event keyword" (7.9.5,
    // receipt f08885e6). PerformActionUsageDeclaration's two alternatives, one clause
    // earlier in the book, told apart the same way on one token: `occurrence` is reserved
    // and a reference opens on a name.
    //
    // The reference alternative's FeatureSpecializationPart may open on a multiplicity
    // (`event subscriptionMessage.source[1];`, 7.9.5), so `[` is asked for as
    // `include_use_case_usage` asks for it. An empty UsageDeclaration builds no node: it
    // is optional, and what follows it, a UsageCompletion, opens on none of its tokens.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Occurrences::Occurrence::timeEnclosedOccurrences, when owned
    //     by an occurrence definition or usage
    // constraint: EventOccurrenceUsage::checkEventOccurrenceUsageSpecialization (8.3.9.2,
    //     receipt 9cf02679). An injection that depends on the owner, so sv2-hir's
    //     (ADR-0002).
    // constraint: EventOccurrenceUsage::validateEventOccurrenceUsageIsReference (8.3.9.2):
    //     `isReference` is derived true whether or not `ref` is written (7.9.5), so the
    //     tree records only what was written.
    // constraint: EventOccurrenceUsage::validateEventOccurrenceUsageReference (8.3.9.2):
    //     the reference's target must be an OccurrenceUsage. A question of resolution, so
    //     sv2-resolve's.
    fn event_occurrence_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EventOccurrenceUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("event");
        if self.at_keyword("occurrence") {
            self.bump_as(keyword("occurrence").unwrap_or(SyntaxKind::BasicName));
            if self.at_name()
                || self.at(SyntaxKind::Lt)
                || self.at_feature_specialization()
                || self.at_multiplicity_part()
            {
                self.usage_declaration();
            }
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        self.usage_completion();
        self.finish_node();
    }

    /// Whether an `EventOccurrenceUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'event'` (`SysML` 8.2.2.9.2). No `def` test, as for
    /// `perform`: `event` names a usage and nothing else. The prefix skipped is the one
    /// `event_occurrence_usage` reads with `occurrence_usage_prefix`.
    pub(super) fn at_event_occurrence_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "event")
    }

    // production: IndividualUsage@sysml
    // production: PortionUsage@sysml
    //
    // IndividualUsage : OccurrenceUsage =
    //     BasicUsagePrefix isIndividual ?= 'individual'
    //     UsageExtensionKeyword* Usage                           (SysML 8.2.2.9.2)
    //
    // PortionUsage : OccurrenceUsage =
    //     BasicUsagePrefix ( isIndividual ?= 'individual' )?
    //     portionKind = PortionKind
    //     UsageExtensionKeyword* Usage
    //     { isPortion = true }                                   (SysML 8.2.2.9.2)
    //
    // The occurrence usages with no kind keyword: "If the declaration of an occurrence
    // usage includes the the [sic] keyword individual (and, possibly, timeslice or
    // snapshot), but no kind keyword, then this is equivalent to having included the
    // occurrence keyword"
    // (7.9.4, receipt 8c84370d), and "timeslice or snapshot may be used in place of the
    // kind keyword" (7.9.3, receipt ebffdbf2). Both metaclasses are OccurrenceUsage
    // (8.3.9.4, receipt bfd9746a).
    //
    // Two productions, one method, told apart by the portion kind: it is what PortionUsage
    // has and IndividualUsage has not. The prefix is written inline, BasicUsagePrefix and
    // not OccurrenceUsagePrefix, so there is no `end` (the EndUsagePrefix alternative is
    // OccurrenceUsagePrefix's alone) and no prefix-metadata before the `individual`.
    //
    // implied specialization: Occurrences::Occurrence::snapshots or ::timeSlices
    // constraint: OccurrenceUsage::checkOccurrenceUsageSnapshotSpecialization and
    //     checkOccurrenceUsageTimeSliceSpecialization (8.3.9.4). Injections, so sv2-hir's
    //     (ADR-0002).
    // constraint: OccurrenceUsage::validateOccurrenceUsagePortionKind (8.3.9.4): a portion
    //     is owned by an occurrence definition or usage ("A time slice or snapshot usage
    //     must be declared in the body of an occurrence definition or usage", 7.9.3). The
    //     grammar reaches it from every body, so it does not guarantee it; sv2-resolve's.
    // constraint: OccurrenceUsage::validateOccurrenceUsageIndividualUsage (8.3.9.4): an
    //     individual usage has an individual definition. A question of resolution, so
    //     sv2-resolve's.
    fn individual_or_portion_usage(&mut self, node: SyntaxKind) {
        self.eat_trivia();
        self.start_node(node);
        if self.at_basic_usage_prefix() {
            self.basic_usage_prefix();
        }
        if node == SyntaxKind::IndividualUsage {
            self.expect_keyword("individual");
        } else {
            self.eat_optional_keyword("individual");
            self.portion_kind();
        }
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.usage();
        self.finish_node();
    }

    /// Which of `IndividualUsage` and `PortionUsage` starts at the `n`th meaningful token,
    /// as the node to build, if either does.
    ///
    /// `BasicUsagePrefix 'individual'? PortionKind?` with at least one of the two keywords
    /// (`SysML` 8.2.2.9.2), its `UsageExtensionKeyword`s, and then the opening of a
    /// `Usage` rather than a kind keyword: before `part`, the same keywords are that
    /// usage's `OccurrenceUsagePrefix`.
    ///
    /// The `nth_opens_usage` test is LOAD-BEARING. `structure_usage_element` asks this
    /// before the keyword `StructureUsageElement`s (`FlowUsage`, `Message`,
    /// `ConnectionUsage`, `InterfaceUsage`, `AllocationUsage`, `ViewUsage`,
    /// `EventOccurrenceUsage` and the rest), so without it `snapshot allocation a;` would
    /// be claimed as a `PortionUsage` where it is an `AllocationUsage` whose
    /// `OccurrenceUsagePrefix` carries the `snapshot` (8.2.2.9.2);
    /// `every_portion_kind_before_a_kind_keyword_is_that_usage_s_prefix` holds it. It is
    /// also asked by `at_sysml_keyword_member` and `at_source_succession_member`.
    pub(super) fn at_individual_or_portion_usage(&self, n: usize) -> Option<SyntaxKind> {
        let k = self.skip_basic_usage_prefix(n);
        let individual = self.nth_is_keyword(k, "individual");
        let k = k + usize::from(individual);
        let portion = self.nth_is_keyword(k, "snapshot") || self.nth_is_keyword(k, "timeslice");
        let after = self.skip_prefix_metadata(k + usize::from(portion));
        if !self.nth_opens_usage(after) {
            None
        } else if portion {
            Some(SyntaxKind::PortionUsage)
        } else if individual {
            Some(SyntaxKind::IndividualUsage)
        } else {
            None
        }
    }

    // production: UsagePrefix@sysml
    //
    // UsagePrefix : Usage = UnextendedUsagePrefix UsageExtensionKeyword*
    //                                                            (SysML 8.2.2.6.2)
    //
    // production: UnextendedUsagePrefix@sysml
    //
    // UnextendedUsagePrefix = EndUsagePrefix | BasicUsagePrefix
    //
    // UnextendedUsagePrefix gets no node, being an alternation whose taken alternative
    // says which it was.
    //
    // The node is built even when empty, as MemberPrefix's is.
    pub(super) fn usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsagePrefix);
        self.unextended_usage_prefix();
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.finish_node();
    }

    /// `UnextendedUsagePrefix = EndUsagePrefix | BasicUsagePrefix` (`SysML` 8.2.2.6.2),
    /// the part of a `UsagePrefix` an `ExtendedUsage` writes before its keywords.
    fn unextended_usage_prefix(&mut self) {
        if self.at_keyword("end") {
            self.end_usage_prefix();
        } else if self.at_basic_usage_prefix() {
            self.basic_usage_prefix();
        }
    }

    // production: OccurrenceUsagePrefix@sysml
    //
    // OccurrenceUsagePrefix : OccurrenceUsage =
    //     ( EndUsagePrefix
    //     | BasicUsagePrefix ( isIndividual ?= 'individual' )?
    //       ( portionKind = PortionKind )?
    //     ) UsageExtensionKeyword*        (SysML 8.2.2.9.2, as deviation OccurrenceUsagePrefix
    //                                      reads it: follow_xtext adds the first alternative)
    //
    // The EndUsagePrefix alternative is read: `end port supplierPort : FuelOutPort;`
    // (training/13. Flows/Flow Definition Example.sysml:8), the deviation's own evidence.
    // The extension keywords follow either alternative.
    //
    // The node is built even when every slot is empty, as MemberPrefix's is.
    pub(super) fn occurrence_usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OccurrenceUsagePrefix);
        if self.at_keyword("end") {
            // The first alternative, by deviation OccurrenceUsagePrefix (follow_xtext):
            // it excludes `individual` and the portion kind.
            // deviation: OccurrenceUsagePrefix
            self.note_deviation("OccurrenceUsagePrefix", "`end` on an occurrence usage");
            self.end_usage_prefix();
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.finish_node();
            return;
        }
        if self.at_basic_usage_prefix() {
            self.basic_usage_prefix();
        }
        if self.at_keyword("individual") {
            self.bump_as(keyword("individual").unwrap_or(SyntaxKind::BasicName));
        }
        if self.at_keyword("snapshot") || self.at_keyword("timeslice") {
            self.portion_kind();
        }
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.finish_node();
    }

    /// Whether any keyword of a `BasicUsagePrefix` is written here.
    fn at_basic_usage_prefix(&self) -> bool {
        [
            "in",
            "out",
            "inout",
            "derived",
            "abstract",
            "variation",
            "constant",
            "ref",
        ]
        .iter()
        .any(|word| self.at_keyword(word))
    }

    // production: BasicUsagePrefix
    //
    // BasicUsagePrefix : Usage = RefPrefix ( isReference ?= 'ref' )?
    //                                                            (SysML 8.2.2.6.2)
    pub(super) fn basic_usage_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicUsagePrefix);
        self.ref_prefix();
        if self.at_keyword("ref") {
            self.bump_as(keyword("ref").unwrap_or(SyntaxKind::BasicName));
        }
        self.finish_node();
    }

    // production: RefPrefix
    //
    // RefPrefix : Usage = ( direction = FeatureDirection )?
    //     ( isDerived ?= 'derived' )?
    //     ( isAbstract ?= 'abstract' | isVariation ?= 'variation' )?
    //     ( isConstant ?= 'constant' )?                          (SysML 8.2.2.6.2)
    //
    // Every part is optional, so the node may be empty — `ref part p;` writes a
    // BasicUsagePrefix whose RefPrefix holds nothing.
    pub(super) fn ref_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RefPrefix);
        if self.at_keyword("in") || self.at_keyword("out") || self.at_keyword("inout") {
            self.feature_direction();
        }
        self.eat_optional_keyword("derived");
        // `isAbstract ?= 'abstract' | isVariation ?= 'variation'` is an alternation,
        // so taking one forecloses the other: `abstract variation part p;` leaves
        // `variation` for the caller to report rather than consuming both.
        if let Some(word) = ["abstract", "variation"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        }
        self.eat_optional_keyword("constant");
        self.finish_node();
    }

    // production: PortionKind
    //
    // PortionKind = 'snapshot' | 'timeslice'                     (SysML 8.2.2.9.2)
    pub(super) fn portion_kind(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PortionKind);
        match ["snapshot", "timeslice"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`snapshot` or `timeslice`"),
        }
        self.finish_node();
    }

    // production: Usage
    //
    // Usage = UsageDeclaration UsageCompletion                   (SysML 8.2.2.6.2)
    pub(super) fn usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Usage);
        self.usage_declaration();
        self.usage_completion();
        self.finish_node();
    }

    // production: UsageDeclaration
    //
    // UsageDeclaration : Usage = Identification FeatureSpecializationPart?
    //                                                            (SysML 8.2.2.6.2)
    pub(super) fn usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageDeclaration);
        self.identification();
        self.optional_feature_specialization_part();
        self.finish_node();
    }

    // production: UsageBody
    //
    // UsageBody : Usage = DefinitionBody                         (SysML 8.2.2.6.2)
    pub(super) fn usage_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageBody);
        self.definition_body();
        self.finish_node();
    }
}
