// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Views, viewpoints and renderings, `SysML` 8.2.2.26.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;

impl Parser<'_> {
    /// Whether a `ViewpointDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'viewpoint' 'def'` (`SysML` 8.2.2.26.3), as
    /// `at_concern_definition` asks of `concern`.
    pub(super) fn at_viewpoint_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "viewpoint") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewpointDefinition@sysml
    //
    // ViewpointDefinition =
    //     OccurrenceDefinitionPrefix 'viewpoint' 'def'
    //     DefinitionDeclaration RequirementBody                  (SysML 8.2.2.26.3)
    //
    // "A viewpoint definition or usage is declared as a kind of requirement definition or
    // usage" (7.26.3, receipt 1813f74a), so RequirementDefinition's spine with the keyword
    // `viewpoint`, as ConcernDefinition is. The Pilot factors the keywords into
    // ViewpointDefKeyword, deviation ViewpointDefKeyword (xtext_only, follow_spec), so the
    // literals are matched here directly. The metaclass is ViewpointDefinition (8.3.26.8,
    // receipt 3cf0e409), a RequirementDefinition.
    //
    // "The subject of a viewpoint definition or usage must be a view" (7.26.3) is a
    // semantic constraint on the subject's type, not a production: any RequirementBody
    // item is read, and checking the subject belongs above this layer (ADR-0002).
    //
    // implied specialization: Views::Viewpoint
    // constraint: ViewpointDefinition::checkViewpointDefinitionSpecialization
    //     `specializesFromLibrary('Views::Viewpoint')` (8.3.26.8). An injection, so
    //     sv2-hir's (ADR-0002).
    pub(super) fn viewpoint_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewpointDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("viewpoint");
        self.expect_keyword("def");
        self.definition_declaration();
        self.requirement_body();
        self.finish_node();
    }

    /// Whether a `ViewDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'view' 'def'` (`SysML` 8.2.2.26.1). `view` is reserved,
    /// so a `viewpoint` is a different token and never answers this.
    pub(super) fn at_view_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "view") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewDefinition@sysml
    //
    // ViewDefinition =
    //     OccurrenceDefinitionPrefix 'view' 'def'
    //     DefinitionDeclaration ViewDefinitionBody               (SysML 8.2.2.26.1)
    //
    // Not on SIMPLE_DEFINITIONS' spine: it names its declaration and its own body rather
    // than taking a `Definition`. The Pilot factors the keywords into ViewDefKeyword,
    // deviation ViewDefKeyword (xtext_only, follow_spec), so the literals are matched here. The metaclass is ViewDefinition (8.3.26.7, receipt
    // ced9a812), a PartDefinition.
    //
    // implied specialization: Views::View
    // constraint: ViewDefinition::checkViewDefinitionSpecialization
    //     `specializesFromLibrary('Views::View')` (8.3.26.7). An injection, so sv2-hir's
    //     (ADR-0002).
    pub(super) fn view_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("view");
        self.expect_keyword("def");
        self.definition_declaration();
        self.view_definition_body();
        self.finish_node();
    }

    // production: ViewDefinitionBody@sysml
    //
    // ViewDefinitionBody : ViewDefinition = ';' | '{' ViewDefinitionBodyItem* '}'
    //                                                            (SysML 8.2.2.26.1)
    //
    // ViewDefinitionBodyItem is marked at `body_element`: DefinitionBodyItem's six
    // alternatives and ElementFilterMember and ViewRenderingMember; see
    // `Body::ViewDefinition`.
    fn view_definition_body(&mut self) {
        self.braced_body(
            SyntaxKind::ViewDefinitionBody,
            Body::ViewDefinition,
            "`;` or `{` after a view definition declaration",
        );
    }

    // production: ViewBody@sysml
    //
    // ViewBody : ViewUsage = ';' | '{' ViewBodyItem* '}'         (SysML 8.2.2.26.2)
    //
    // ViewBodyItem is marked at `body_element`: ViewDefinitionBodyItem's alternatives and
    // Expose; see `Body::View`.
    fn view_body(&mut self) {
        self.braced_body(
            SyntaxKind::ViewBody,
            Body::View,
            "`;` or `{` after a view usage declaration",
        );
    }

    // production: ViewRenderingMember@sysml
    //
    // ViewRenderingMember : ViewRenderingMembership =
    //     MemberPrefix 'render'
    //     ownedRelatedElement += ViewRenderingUsage              (SysML 8.2.2.26.1)
    //
    // The metaclass is ViewRenderingMembership (8.3.26.10, receipt 75857b4b), a
    // FeatureMembership whose referencedRendering is the reference's target when the
    // usage has one and the usage itself otherwise.
    //
    // constraint: ViewRenderingMembership::validateViewRenderingMembershipOwningType
    //     (8.3.26.10): the owner is a view. The grammar already reaches this member from
    //     the two view bodies alone.
    pub(super) fn view_rendering_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewRenderingMember);
        self.member_prefix();
        self.expect_keyword("render");
        self.view_rendering_usage();
        self.finish_node();
    }

    // production: ViewRenderingUsage@sysml
    //
    // ViewRenderingUsage : RenderingUsage =
    //       ownedRelationship += OwnedReferenceSubsetting
    //       FeatureSpecializationPart?
    //       UsageBody
    //     | ( UsageExtensionKeyword* 'rendering'
    //       | UsageExtensionKeyword+ )
    //       Usage                                                (SysML 8.2.2.26.1)
    //
    // FramedConcernUsage's and RequirementConstraintUsage's shape: a rendering by
    // reference (`render asTreeDiagram;`, training/42. Views/Views Example.sysml:13), or
    // declared (`render rendering r1: R[0..1];`, examples/Simple Tests/ViewTest.sysml:32).
    // The alternatives are told apart before either begins: the second opens on the
    // keyword `rendering` or a `#`, the first on a name, and a keyword is not a name
    // (8.2.2.1.2). `( X* 'rendering' | X+ )` is "a `#` or a `rendering`", then the rest.
    fn view_rendering_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewRenderingUsage);
        if self.at_keyword("rendering") || self.at(SyntaxKind::Hash) {
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
            self.eat_optional_keyword("rendering");
            self.usage();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
            self.usage_body();
        }
        self.finish_node();
    }

    /// Whether a `ViewUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'view'` with no `def` after it (`SysML` 8.2.2.26.2).
    pub(super) fn at_view_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "view") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewUsage@sysml
    //
    // ViewUsage =
    //     OccurrenceUsagePrefix 'view'
    //     UsageDeclaration? ValuePart? ViewBody                  (SysML 8.2.2.26.2)
    //
    // A StructureUsageElement (8.2.2.6.4). The declaration is optional whole, so
    // `view { ... }` declares nothing and `view :>> columnView[1] { ... }` (training/42.
    // Views/Views Example.sysml:17) only a redefinition; it is read only when something
    // that opens one is written, as `event_occurrence_usage` reads its own, and an empty one
    // builds no node. The Pilot factors the keyword into ViewUsageKeyword, deviation
    // ViewUsageKeyword (xtext_only, follow_spec), so the literal is matched here. The metaclass is ViewUsage (8.3.26.11, receipt 6bbae03d), a PartUsage.
    //
    // implied specialization: Views::views, and Views::View::subviews when owned by a view
    // constraint: ViewUsage::checkViewUsageSpecialization and
    //     checkViewUsageSubviewSpecialization (8.3.26.11). Injections, so sv2-hir's
    //     (ADR-0002).
    pub(super) fn view_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("view");
        if self.at_name()
            || self.at(SyntaxKind::Lt)
            || self.at_feature_specialization()
            || self.at_multiplicity_part()
        {
            self.usage_declaration();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.view_body();
        self.finish_node();
    }

    /// Whether a `ViewpointUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'viewpoint'` with no `def` after it (`SysML` 8.2.2.26.3), as
    /// `at_concern_usage` asks of `concern`.
    pub(super) fn at_viewpoint_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "viewpoint") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: ViewpointUsage@sysml
    //
    // ViewpointUsage =
    //     OccurrenceUsagePrefix 'viewpoint'
    //     ConstraintUsageDeclaration RequirementBody             (SysML 8.2.2.26.3)
    //
    // RequirementUsage's shape with the kind keyword `viewpoint` (7.26.3, receipt
    // 1813f74a); deviation ViewpointUsageKeyword (xtext_only, follow_spec) matches the
    // literal. The metaclass is ViewpointUsage (8.3.26.9, receipt 26897969), a
    // RequirementUsage. A BehaviorUsageElement (8.2.2.6.4), as ConcernUsage is.
    //
    // implied specialization: Views::viewpoints, and Views::View::viewpointSatisfactions
    //     when composite and owned by a view
    // constraint: ViewpointUsage::checkViewpointUsageSpecialization and
    //     checkViewpointUsageViewpointSatisfactionSpecialization (8.3.26.9). Injections, so
    //     sv2-hir's (ADR-0002).
    pub(super) fn viewpoint_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ViewpointUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("viewpoint");
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }
}
