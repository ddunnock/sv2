// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The `KerML` kernel elements written with their own keyword, `KerML` 8.2.5: steps,
//! expressions, invariants, functions, connectors, binding connectors, successions and flows.

use crate::generated::kinds::SyntaxKind;
use crate::parser::lookahead::keyword;
use crate::parser::{Body, Parser};

/// The reserved words a `KerML` `FeaturePrefix` stands before, one per `FeatureElement`
/// but `Feature`'s keywordless alternative (`KerML` 8.2.3.4.3, 8.2.4.3.1, 8.2.5): the
/// keyword that ends an `end` feature's `OwnedCrossFeatureMember`
/// (`skip_kerml_end_feature_prefix`). Listed whether implemented or not, since the
/// cross feature ends at one either way.
pub(super) const KERML_FEATURE_ELEMENT_KEYWORDS: [&str; 9] = [
    "feature",
    "step",
    "expr",
    "bool",
    "inv",
    "connector",
    "binding",
    "succession",
    "flow",
];

/// Which of `KerML` `Connector`'s three declaration forms is written (`KerML` 8.2.5.5.1).
#[derive(Clone, Copy)]
enum ConnectorForm {
    /// `FeatureDeclaration? ValuePart?`.
    Feature,
    /// `BinaryConnectorDeclaration`, `from ... to ...`.
    Binary,
    /// `NaryConnectorDeclaration`, `( ... , ... )`.
    Nary,
}

impl Parser<'_> {
    /// Whether a `KerML` `Succession` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `succession`. The keyword is reserved (`KerML` 8.2.2.6),
    /// so it decides on its own, and `at_feature` never claims it as a name. The one
    /// other production that opens the same way is `SuccessionFlow`, `succession flow`
    /// (8.2.5.9.2): unimplemented, and declined here so that it is reported rather than
    /// read as a succession whose declaration names `flow`.
    pub(super) fn at_kerml_succession(&self, n: usize) -> bool {
        let n = self.skip_feature_prefix(n);
        self.nth_is_keyword(n, "succession") && !self.nth_is_keyword(n + 1, "flow")
    }

    // production: Succession@kerml
    //
    // Succession : Succession =
    //     FeaturePrefix 'succession' SuccessionDeclaration TypeBody   (KerML 8.2.5.5.3)
    //
    // Scoped `kerml`: SysML writes its succession as SuccessionAsUsage (8.2.2.13.3), over
    // UsagePrefix and UsageDeclaration, and a .sysml file never reaches this (ADR-0014).
    // The two share ConnectorEndMember and nothing else, so the Rust name is scoped too.
    //
    // FeaturePrefix is read whole (`feature_prefix`), as this production's own parts are.
    //
    // implied specialization: Occurrences::happensBeforeLinks
    // constraint: Succession::checkSuccessionSpecialization
    //     `specializesFromLibrary('Occurrences::happensBeforeLinks')` (KerML 8.3.4.5.4).
    //     An injection, so sv2-hir's; this layer builds the tree only (ADR-0002).
    pub(super) fn kerml_succession(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Succession);
        self.feature_prefix();
        self.expect_keyword("succession");
        self.succession_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: SuccessionDeclaration@kerml
    //
    // SuccessionDeclaration : Succession =
    //     FeatureDeclaration
    //       ( 'first' ownedRelationship += ConnectorEndMember
    //         'then' ownedRelationship += ConnectorEndMember )?
    //   | ( isSufficient ?= 'all' )?
    //       ( 'first'? ownedRelationship += ConnectorEndMember
    //         'then' ownedRelationship += ConnectorEndMember )?      (KerML 8.2.5.5.3)
    //
    // The clause writes `s.isSufficient`; the stray `s.` names a property and consumes
    // no token, so the rule is the Xtext fragment's (recorded on the unit).
    //
    // The two alternatives are told apart before either commits, on what follows an
    // optional `all`. The second is taken when a `first` is written there, when nothing
    // is (the empty declaration: `succession;`, `succession { }`), or when a ConnectorEnd
    // is written there and a `then` directly after it (`succession a then b;`). Anything
    // else is a FeatureDeclaration: `succession s first a then b;`, `succession : T;`.
    // The cases are disjoint — `first` and `then` are reserved, so no FeatureDeclaration
    // opens on `first`, and a FeatureDeclaration followed by `then` is no succession.
    // `all` is left to FeatureDeclaration's own slot in the first alternative.
    //
    // The node is built even when empty, as FeaturePrefix's is.
    fn succession_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuccessionDeclaration);
        let after_all = usize::from(self.at_keyword("all"));
        let empty = self.nth_is(after_all, SyntaxKind::Semicolon)
            || self.nth_is(after_all, SyntaxKind::LBrace)
            || self.peek_nth(after_all).is_none();
        let ends = self.nth_is_keyword(after_all, "first")
            || self
                .skip_connector_end(after_all)
                .is_some_and(|after| self.nth_is_keyword(after, "then"));
        if empty || ends {
            self.eat_optional_keyword("all");
            if ends {
                self.eat_optional_keyword("first");
                self.connector_end_member();
                self.expect_keyword("then");
                self.connector_end_member();
            }
        } else {
            self.feature_declaration();
            if self.at_keyword("first") {
                self.bump_as(keyword("first").unwrap_or(SyntaxKind::BasicName));
                self.connector_end_member();
                self.expect_keyword("then");
                self.connector_end_member();
            }
        }
        self.finish_node();
    }

    /// Whether a `KerML` `Step` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `step`, reserved (`KerML` 8.2.2.6) and opening no other
    /// production, so it decides on its own.
    pub(super) fn at_kerml_step(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "step")
    }

    // production: Step@kerml
    //
    // Step : Step =
    //     FeaturePrefix
    //     'step' FeatureDeclaration ValuePart?
    //     TypeBody                                                (KerML 8.2.5.6.2)
    //
    // Scoped `kerml`: SysML has no `step`; its steps are ActionUsages (SysML 8.2.2.17),
    // and a .sysml file never reaches this (ADR-0014). The metaclass is Step (8.3.4.6.3,
    // receipt 873de1f5), a Feature typed by Behaviors, so the shape is Feature's first
    // alternative with its own keyword: `step paint : Paint [1];` (KerML Spec Annex A
    // Examples/A-3-6-Sequences.kerml:8).
    //
    // The FeatureDeclaration is optional here although the clause writes it bare: deviation
    // Step (follow_xtext), extrapolated from Expression's KERML11-181 and from Connector's
    // written `FeatureDeclaration?` (8.2.5.5.1). So `step;` parses, and carries a
    // PARSE-DEVIATION note, since only the deviation admits it (ADR-0022).
    //
    // Marked although FeaturePrefix and FeatureDeclaration are not, as Connector is.
    //
    // implied specialization: Performances::performances, and
    //     Performance::enclosedPerformance, Performance::subperformance or
    //     Object::ownedPerformance by the step's owner
    // constraint: Step::checkStepSpecialization, checkStepEnclosedPerformanceSpecialization,
    //     checkStepSubperformanceSpecialization and checkStepOwnedPerformanceSpecialization
    //     (KerML 8.3.4.6.3, receipt 873de1f5; semantics 8.4.4.7.2, receipt 390500e6).
    //     Injections, so sv2-hir's; this layer builds the tree only (ADR-0002).
    pub(super) fn kerml_step(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Step);
        self.feature_prefix();
        self.expect_keyword("step");
        if self.at_feature_declaration() {
            self.feature_declaration();
        } else {
            // deviation: Step
            self.note_deviation("Step", "a step with no declaration");
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.type_body();
        self.finish_node();
    }

    /// Whether a `KerML` `Invariant` starts at the `n`th meaningful token: a
    /// `FeaturePrefix`, then `inv`, reserved (`KerML` 8.2.2.6).
    pub(super) fn at_kerml_invariant(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "inv")
    }

    // production: Invariant@kerml
    //
    // Invariant : Invariant =
    //     FeaturePrefix
    //     'inv' ( 'true' | isNegated ?= 'false' )?
    //     FeatureDeclaration ValuePart?
    //     FunctionBody                                           (KerML 8.2.5.7.4)
    //
    // A BooleanExpression asserted true, or with `false` negated (8.3.4.7.5, receipt
    // 45001822): "declared like any other boolean expression, except using the keyword
    // inv instead of bool, and, additionally, this keyword may be optionally followed by
    // one of the keywords true or false" (7.4.8.5, receipt 1dd5b04f).
    //
    // The FeatureDeclaration is optional although the clause writes it bare: deviation
    // Invariant (follow_xtext), OMG issue KERML11-181, which names this clause, and the
    // corpus writes `inv { age >= 35 }` (Individuals
    // Examples/JohnIndividualExample.kerml:89). So that text parses and carries a
    // PARSE-DEVIATION note, since only the deviation admits it (ADR-0022).
    //
    // implied specialization: Performances::trueEvaluations, or falseEvaluations when
    //     negated
    // constraint: Invariant::checkInvariantSpecialization (KerML 8.3.4.7.5). An
    //     injection, so sv2-hir's; nothing is written into the tree.
    pub(super) fn kerml_invariant(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Invariant);
        self.feature_prefix();
        self.expect_keyword("inv");
        self.eat_one_of(&["true", "false"]);
        if self.at_feature_declaration() {
            self.feature_declaration();
        } else {
            // deviation: Invariant
            self.note_deviation("Invariant", "an invariant with no declaration");
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.function_body();
        self.finish_node();
    }

    /// Whether a `KerML` `Expression` starts at the `n`th meaningful token: a
    /// `FeaturePrefix`, then `expr`, reserved (`KerML` 8.2.2.6).
    pub(super) fn at_kerml_expression(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "expr")
    }

    /// Whether a `KerML` `BooleanExpression` starts at the `n`th meaningful token: a
    /// `FeaturePrefix`, then `bool`, reserved (`KerML` 8.2.2.6).
    pub(super) fn at_kerml_boolean_expression(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "bool")
    }

    // production: Expression@kerml
    //
    // Expression : Expression =
    //     FeaturePrefix
    //     'expr' FeatureDeclaration ValuePart?
    //     FunctionBody                                           (KerML 8.2.5.7.2)
    //
    // A Step typed by Functions (8.3.4.7.3, receipt 9df44f16), "declared as a step ...
    // using the keyword expr" with a function's body (7.4.8.3, receipt a4252270):
    // `expr totalMass: TotalMass { in mass; in sub; }` (Simple Tests/Expressions.kerml:50).
    //
    // The FeatureDeclaration is optional although the clause writes it bare: deviation
    // Expression (follow_xtext), KERML11-181, which names this clause. So `expr { 1 }`
    // parses and carries a PARSE-DEVIATION note (ADR-0022), as Invariant's does.
    //
    // implied specialization: Performances::evaluations
    // constraint: Expression::checkExpressionSpecialization and the result constraints
    //     of 8.3.4.7.3 (checkExpressionResultBindingConnector,
    //     validateExpressionResultExpressionMembership,
    //     validateExpressionResultParameterMembership). Injections and validity,
    //     sv2-hir's and sv2-resolve's; nothing is written into the tree.
    pub(super) fn kerml_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Expression);
        self.feature_prefix();
        self.expect_keyword("expr");
        if self.at_feature_declaration() {
            self.feature_declaration();
        } else {
            // deviation: Expression
            self.note_deviation("Expression", "an expression with no declaration");
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.function_body();
        self.finish_node();
    }

    // production: BooleanExpression@kerml
    //
    // BooleanExpression : BooleanExpression =
    //     FeaturePrefix
    //     'bool' FeatureDeclaration ValuePart?
    //     FunctionBody                                           (KerML 8.2.5.7.4)
    //
    // "A boolean expression is declared as an expression ..., using the keyword bool"
    // (7.4.8.5, receipt 1dd5b04f); the metaclass is BooleanExpression (8.3.4.7.2,
    // receipt fe24fc2f). Invariant's shape less its `true`/`false`. The declaration's `?`
    // is deviation BooleanExpression's (follow_xtext, KERML11-181), noted as Expression's
    // is; the register records that no corpus file exercises the anonymous form.
    //
    // implied specialization: Performances::booleanEvaluations
    // constraint: BooleanExpression::checkBooleanExpressionSpecialization (KerML
    //     8.3.4.7.2). An injection, sv2-hir's.
    pub(super) fn kerml_boolean_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BooleanExpression);
        self.feature_prefix();
        self.expect_keyword("bool");
        if self.at_feature_declaration() {
            self.feature_declaration();
        } else {
            // deviation: BooleanExpression
            self.note_deviation(
                "BooleanExpression",
                "a boolean expression with no declaration",
            );
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.function_body();
        self.finish_node();
    }

    /// Which of `KerML`'s `Flow` and `SuccessionFlow` starts at the `n`th meaningful
    /// token, if either does: `flow`, or `succession flow`, after a `FeaturePrefix`. Its
    /// node, and whether it is the succession. `at_kerml_succession` declines the pair.
    pub(super) fn at_kerml_flow(&self, n: usize) -> Option<(SyntaxKind, bool)> {
        let after = self.skip_feature_prefix(n);
        if self.nth_is_keyword(after, "flow") {
            Some((SyntaxKind::Flow, false))
        } else if self.nth_is_keyword(after, "succession") && self.nth_is_keyword(after + 1, "flow")
        {
            Some((SyntaxKind::SuccessionFlow, true))
        } else {
            None
        }
    }

    // production: Flow@kerml
    // production: SuccessionFlow@kerml
    //
    // Flow           = FeaturePrefix 'flow' FlowDeclaration TypeBody
    // SuccessionFlow = FeaturePrefix 'succession' 'flow' FlowDeclaration TypeBody
    //                                                            (KerML 8.2.5.9.2)
    //
    // "A flow declaration is syntactically similar to a binary connector declaration ...,
    // using the keyword flow, or succession flow for a succession flow" (7.4.10.3,
    // receipt 95392e97). A Flow is a Step and a Connector (8.3.4.9.2, receipt 2be03181);
    // a SuccessionFlow a Flow and a Succession (8.3.4.9.6, receipt cc36307d). One shape,
    // one method, as SysML's FlowUsage and SuccessionFlowUsage share `flow_declaration`.
    // The Pilot factors the keywords (FlowKeyword, SuccessionFlowKeyword); both are
    // xtext_only, follow_spec, so the literals are matched here.
    //
    // implied specialization: Transfers::transfers; Transfers::flowTransfers when the flow
    //     has owned end features; Transfers::flowTransfersBefore for a succession flow.
    //     7.4.10.3's prose names the default subsetting `transfersBefore`; the constraints
    //     name the library element, and are what sv2-hir reads.
    // constraint: Flow::checkFlowSpecialization (Transfers::transfers) and
    //     checkFlowWithEndsSpecialization (Transfers::flowTransfers, KerML 8.3.4.9.2);
    //     SuccessionFlow::checkSuccessionFlowSpecialization (Transfers::flowTransfersBefore,
    //     8.3.4.9.6). Injections, sv2-hir's; nothing is written into the tree.
    pub(super) fn kerml_flow(&mut self, (node, succession): (SyntaxKind, bool)) {
        self.eat_trivia();
        self.start_node(node);
        self.feature_prefix();
        if succession {
            self.expect_keyword("succession");
        }
        self.expect_keyword("flow");
        self.kerml_flow_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: FlowDeclaration@kerml
    //
    // FlowDeclaration : Flow =
    //       FeatureDeclaration ValuePart?
    //       ( 'of'  ownedRelationship += PayloadFeatureMember )?
    //       ( 'from' ownedRelationship += FlowEndMember
    //         'to'   ownedRelationship += FlowEndMember )?
    //     | ( isSufficient ?= 'all' )?
    //       ownedRelationship += FlowEndMember 'to'
    //       ownedRelationship += FlowEndMember                    (KerML 8.2.5.9.2)
    //
    // The same node as SysML's FlowDeclaration (8.2.2.16), a production of the same name
    // over UsageDeclaration and FlowPayloadFeatureMember, and with no `all`. The
    // alternatives are told apart as SysML's are, by looking past a whole flow end, after
    // any `all`, for the `to` only the second writes there.
    //
    // The first alternative's FeatureDeclaration is optional although the clause writes
    // it bare: deviation FlowDeclaration (follow_xtext), which the register records as
    // extrapolated from KERML11-181. 7.4.10.3's own example writes one without it, `flow
    // of flowingFuel : Fuel from fuelTank.fuelOut to engine.fuelIn;` (receipt
    // 95392e97). Such text parses and carries a PARSE-DEVIATION note (ADR-0022).
    fn kerml_flow_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowDeclaration);
        let n = usize::from(self.at_keyword("all"));
        let ends_first = self
            .flow_end_segments(n)
            .is_some_and(|(after, _)| self.nth_is_keyword(after, "to"));
        if ends_first {
            self.eat_optional_keyword("all");
            self.flow_end_member();
            self.expect_keyword("to");
            self.flow_end_member();
        } else {
            if self.at_feature_declaration() {
                self.feature_declaration();
            } else {
                // deviation: FlowDeclaration
                self.note_deviation("FlowDeclaration", "a flow with no declaration");
            }
            if self.at_value_part() {
                self.value_part();
            }
            if self.at_keyword("of") {
                self.expect_keyword("of");
                self.payload_feature_member();
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

    // production: PayloadFeatureMember@kerml
    //
    // PayloadFeatureMember : FeatureMembership =
    //     ownedRelatedElement = PayloadFeature                   (KerML 8.2.5.9.2)
    //
    // KerML's, with no FlowPayloadFeature between, where SysML's FlowPayloadFeatureMember
    // has one.
    fn payload_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadFeatureMember);
        self.kerml_payload_feature();
        self.finish_node();
    }

    // production: PayloadFeature@kerml
    //
    // PayloadFeature : PayloadFeature =
    //       Identification PayloadFeatureSpecializationPart ValuePart?
    //     | Identification ValuePart
    //     | ownedRelationship += OwnedFeatureTyping
    //       ( ownedRelationship += OwnedMultiplicity )?
    //     | ownedRelationship += OwnedMultiplicity
    //       ownedRelationship += OwnedFeatureTyping             (KerML 8.2.5.9.2)
    //
    // As the pinned transcription and SysML 8.2.2.16 state it, by deviation
    // PayloadFeature (follow_spec), with the KerML-only second alternative kept: SysML's
    // three and `Identification ValuePart`, a payload named and valued with no typing,
    // `of p = 1`. The first two open alike and are told apart by what follows the
    // Identification: a FeatureSpecialization (`payload_feature_is_declared`, as SysML's)
    // or a value. The metaclass is PayloadFeature (8.3.4.9.5, receipt d49a93fa).
    //
    // The valued alternative is asked FIRST: `payload_feature_is_declared` answers yes on
    // any leading `<`, which is sound in SysML, where only the declared alternative opens
    // on a short name, and not here, where `of <p> = 1` is the valued one.
    fn kerml_payload_feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PayloadFeature);
        if self.payload_feature_is_valued() {
            self.identification();
            self.value_part();
        } else if self.payload_feature_is_declared() {
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

    /// Whether the payload here is `KerML`'s `Identification ValuePart` alternative: an
    /// optional `<short name>` and a name, then a `FeatureValue`'s `=`, `:=` or
    /// `default` (8.2.5.10).
    fn payload_feature_is_valued(&self) -> bool {
        let mut n = 0;
        if self.nth_is(n, SyntaxKind::Lt) {
            n += 3;
        }
        n += usize::from(self.nth_is_name(n));
        self.nth_is(n, SyntaxKind::Eq)
            || self.nth_is(n, SyntaxKind::ColonEq)
            || self.nth_is_keyword(n, "default")
    }

    // production: FunctionBody@kerml
    //
    // FunctionBody : Type = ';' | '{' FunctionBodyPart '}'       (KerML 8.2.5.7.1)
    //
    // The body of a Function, a Predicate, an Expression, a BooleanExpression and an
    // Invariant; the Invariant reaches it here. "The body of a function is like the body
    // of a behavior ..., with the optional addition of the declaration of a result
    // expression at the end" (7.4.8.2, receipt 29d9c85d).
    fn function_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FunctionBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.function_body_part();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` to open a function body");
        }
        self.finish_node();
    }

    // production: FunctionBodyPart@kerml
    //
    // FunctionBodyPart : Type =
    //     ( TypeBodyElement
    //     | ownedRelationship += ReturnFeatureMember
    //     )*
    //     ( ownedRelationship += ResultExpressionMember )?       (KerML 8.2.5.7.1)
    //
    // SysML's CalculationBodyPart in KerML's items: the star is greedy and the expression
    // is last, with no `;` ("A result expression is written without a final semicolon",
    // 7.4.8.2), so the one question is where the run ends, and `at_kerml_result_expression`
    // answers it. Body::Function reads the items.
    pub(super) fn function_body_part(&mut self) {
        // A leading `/* ... */` is the run's first member, so it is left for the loop.
        self.with_significant_comments(Self::eat_trivia);
        self.start_node(SyntaxKind::FunctionBodyPart);
        self.body_elements(Some(SyntaxKind::RBrace), Body::Function);
        if !self.at_end() && !self.at(SyntaxKind::RBrace) {
            self.result_expression_member();
        }
        self.finish_node();
    }

    // production: ReturnFeatureMember@kerml
    //
    // ReturnFeatureMember : ReturnParameterMembership =
    //     MemberPrefix 'return'
    //     ownedRelatedElement += FeatureElement                  (KerML 8.2.5.7.1)
    //
    // The result parameter, "declared in its body by beginning the declaration with the
    // keyword return (instead of a direction keyword)" (7.4.8.2, receipt 29d9c85d):
    // `return : Rational;`, a keywordless Feature declared by its typing. SysML's
    // ReturnParameterMember owns a UsageElement where this owns a FeatureElement.
    //
    // constraint: ReturnParameterMembership::validateReturnParameterMembershipOwningType
    //     (KerML 8.3.4.7.8, receipt 259909e5), and the parameter's direction `out`, which
    //     the text does not write: an injection, sv2-hir's, as for SysML's member.
    pub(super) fn return_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ReturnFeatureMember);
        self.member_prefix();
        self.expect_keyword("return");
        if self.at_kerml_keyword_feature_element(0) || self.at_feature(0) {
            self.feature_element();
        } else {
            self.error_expected("a feature after `return`");
        }
        self.finish_node();
    }

    /// Whether a `KerML` `Connector` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `connector`, which is reserved (`KerML` 8.2.2.6) and opens
    /// no other `KerML` production, so it decides on its own.
    pub(super) fn at_kerml_connector(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "connector")
    }

    // production: Connector@kerml
    //
    // Connector : Connector =
    //     FeaturePrefix 'connector'
    //     ( FeatureDeclaration? ValuePart?
    //     | ConnectorDeclaration
    //     )
    //     TypeBody                                                (KerML 8.2.5.5.1)
    //
    // production: ConnectorDeclaration@kerml
    //
    // ConnectorDeclaration : Connector =
    //     BinaryConnectorDeclaration | NaryConnectorDeclaration  (KerML 8.2.5.5.1)
    //
    // Scoped `kerml`: SysML's connector is ConnectionUsage (8.2.2.13.1), keyword
    // `connection`/`connect`, and a .sysml file never reaches this (ADR-0014). The
    // metaclass is Connector (8.3.4.5.3, receipt 8ac87fd0). ConnectorDeclaration is an
    // alternation with no node, as ControlNode is; the member node says which.
    //
    // All three alternatives may open on a FeatureDeclaration, so the choice is made by
    // looking along the statement, which `connector_form` does: a `from` anywhere, or an
    // end followed directly by `to`, is the binary form; a `(` before any `;`, brace or
    // `=` is the n-ary; anything else is the first alternative. `to`, `=` and `(` are not
    // in a FeatureDeclaration -- only a ValuePart's expression writes a `(`, and the
    // n-ary form has no ValuePart, so a `(` after `=` is the first alternative's. `from`
    // IS, in one place: FeatureRelationshipPart reaches DisjoiningPart, `'disjoint' 'from'
    // OwnedDisjoining` (KerML 8.2.4.1.1). So a `from` directly after `disjoint` is not
    // counted, and `connector c disjoint from d;` is the first alternative.
    //
    // Marked although FeaturePrefix is not, for the reason Succession gives, and although
    // FeatureDeclaration is not: its implemented part is what is read here, as Feature
    // reads it.
    //
    // implied specialization: Links::links, Links::binaryLinks for two ends, and the
    //     Objects:: forms for an AssociationStructure type
    // constraint: Connector::checkConnectorSpecialization,
    //     checkConnectorBinarySpecialization, checkConnectorObjectSpecialization and
    //     checkConnectorBinaryObjectSpecialization (KerML 8.3.4.5.3). Injections, so
    //     sv2-hir's; this layer builds the tree only (ADR-0002).
    pub(super) fn kerml_connector(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Connector);
        self.feature_prefix();
        self.expect_keyword("connector");
        match self.connector_form() {
            ConnectorForm::Binary => self.binary_connector_declaration(),
            ConnectorForm::Nary => self.nary_connector_declaration(),
            ConnectorForm::Feature => {
                if self.at_feature_declaration() {
                    self.feature_declaration();
                }
                if self.at_value_part() {
                    self.value_part();
                }
            }
        }
        self.type_body();
        self.finish_node();
    }

    /// Which of `Connector`'s three declaration forms is written here, after the
    /// `connector` keyword: see `kerml_connector`.
    fn connector_form(&self) -> ConnectorForm {
        if self.connector_from_follows() {
            return ConnectorForm::Binary;
        }
        let after_all = usize::from(self.at_keyword("all"));
        if self
            .skip_connector_end(after_all)
            .is_some_and(|after| self.nth_is_keyword(after, "to"))
        {
            return ConnectorForm::Binary;
        }
        let mut n = 0;
        loop {
            if self.nth_is(n, SyntaxKind::LParen) {
                return ConnectorForm::Nary;
            }
            if self.peek_nth(n).is_none()
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::LBrace)
                || self.nth_is(n, SyntaxKind::RBrace)
                || self.nth_is(n, SyntaxKind::Eq)
            {
                return ConnectorForm::Feature;
            }
            n += 1;
        }
    }

    /// Whether the statement from here writes a binary connector's `from`: a `from`
    /// before its `;` or brace that is not `disjoint from` (see `kerml_connector`).
    fn connector_from_follows(&self) -> bool {
        let mut n = 0;
        while let Some(from) = self.scan_for_keyword(n, "from") {
            if from == 0 || !self.nth_is_keyword(from - 1, "disjoint") {
                return true;
            }
            n = from + 1;
        }
        false
    }

    // production: BinaryConnectorDeclaration@kerml
    //
    // BinaryConnectorDeclaration : Connector =
    //     ( FeatureDeclaration? 'from' | isSufficient ?= 'all' 'from'? )?
    //     ownedRelationship += ConnectorEndMember 'to'
    //     ownedRelationship += ConnectorEndMember                  (KerML 8.2.5.5.1)
    //
    // "the source related feature is referenced after the keyword from, and the target
    // related feature is referenced after the keyword to ... If a binary connector
    // declaration includes only the related features part, then the keyword from can be
    // omitted" (7.4.6.2, receipt d8abbbc3). `all from` is the second alternative's; `all`
    // with a declaration after it is the FeatureDeclaration's own `all`, since a
    // FeatureDeclaration cannot be `all` alone.
    fn binary_connector_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BinaryConnectorDeclaration);
        if !self.connector_from_follows() {
            // No `from`: the ends alone, or `all` and the ends.
            self.eat_optional_keyword("all");
        } else if self.at_keyword("all") && self.nth_is_keyword(1, "from") {
            // `all from`: the second alternative with its optional `from` written.
            self.eat_optional_keyword("all");
            self.expect_keyword("from");
        } else {
            // `FeatureDeclaration? 'from'`: whatever stands before the `from` declares.
            if !self.at_keyword("from") {
                self.feature_declaration();
            }
            self.expect_keyword("from");
        }
        self.connector_end_member();
        self.expect_keyword("to");
        self.connector_end_member();
        self.finish_node();
    }

    // production: NaryConnectorDeclaration@kerml
    //
    // NaryConnectorDeclaration : Connector =
    //     FeatureDeclaration?
    //     '(' ownedRelationship += ConnectorEndMember ','
    //         ownedRelationship += ConnectorEndMember
    //         ( ',' ownedRelationship += ConnectorEndMember )*
    //     ')'                                                     (KerML 8.2.5.5.1)
    //
    // "they can be listed between parentheses, after the regular feature declaration part
    // and before the body of the connector" (7.4.6.2, receipt d8abbbc3). At least two
    // ends: a list of one is no connector's.
    fn nary_connector_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NaryConnectorDeclaration);
        if !self.at(SyntaxKind::LParen) {
            self.feature_declaration();
        }
        self.expect(SyntaxKind::LParen, "`(`");
        self.connector_end_member();
        self.expect(SyntaxKind::Comma, "`,` and a second connector end");
        self.connector_end_member();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.connector_end_member();
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    /// Whether a `KerML` `BindingConnector` starts at the `n`th meaningful token.
    ///
    /// A `FeaturePrefix`, then `binding`. The keyword is reserved (`KerML` 8.2.2.6) and
    /// opens no other `KerML` production, so it decides on its own, and `at_feature` never
    /// claims it as a name. The prefix skipped is `FeaturePrefix`, which is what
    /// `kerml_binding_connector` reads: a recogniser that looked past more than its
    /// production consumes would make the body loop ask again at the same token.
    pub(super) fn at_kerml_binding_connector(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_feature_prefix(n), "binding")
    }

    // production: BindingConnector@kerml
    //
    // BindingConnector : BindingConnector =
    //     FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody   (KerML 8.2.5.5.2)
    //
    // Scoped `kerml`, as Succession is: SysML writes its binding as
    // BindingConnectorAsUsage (8.2.2.13.2), over UsagePrefix and `bind`, and a .sysml file
    // never reaches this (ADR-0014). The two share ConnectorEndMember and nothing else,
    // so the Rust name is scoped too. Receipt 02578996 is the production's.
    //
    // Marked although FeaturePrefix is not, for the reason Succession gives.
    //
    // implied specialization: Links::selfLinks
    // constraint: BindingConnector::checkBindingConnectorSpecialization
    //     `specializesFromLibrary('Links::selfLinks')` (KerML 8.3.4.5.2), which "requires
    //     that BindingConnectors specialize the Feature Links::selfLinks" (8.4.4.6.2,
    //     receipt c87b1db0). An injection, so sv2-hir's; this layer builds the tree only
    //     (ADR-0002).
    // constraint: BindingConnector::validateBindingConnectorIsBinary
    //     `relatedFeature->size() = 2` (KerML 8.3.4.5.2). NOT held by this grammar, unlike
    //     SysML's: the declaration's ends are optional, so `binding;` and a binding whose
    //     ends are declared as end features in its body both parse, and whether there are
    //     two is a question for the layer that counts relatedFeatures (ADR-0002: validity
    //     gates writes, never reads).
    pub(super) fn kerml_binding_connector(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnector);
        self.feature_prefix();
        self.expect_keyword("binding");
        self.binding_connector_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: BindingConnectorDeclaration@kerml
    //
    // BindingConnectorDeclaration : BindingConnector =
    //     FeatureDeclaration
    //       ( 'of' ownedRelationship += ConnectorEndMember
    //         '=' ownedRelationship += ConnectorEndMember )?
    //   | ( isSufficient ?= 'all' )?
    //       ( 'of'? ownedRelationship += ConnectorEndMember
    //         '=' ownedRelationship += ConnectorEndMember )?          (KerML 8.2.5.5.2)
    //
    // SuccessionDeclaration's shape, `of` for `first` and `=` for `then`, and told apart
    // the same way on what follows an optional `all`: the second alternative when `of` is
    // written there, when nothing is (`binding;`, `binding { ... }`), or when a
    // ConnectorEnd is and `=` directly after it (`binding a = b;`); a FeatureDeclaration
    // otherwise (`binding ab of a = b;`, `binding : T;`). Disjoint because `of` is
    // reserved (8.2.2.6), so no FeatureDeclaration opens on it, and a FeatureDeclaration
    // takes no `=` here — this production writes no ValuePart. "If a binding connector
    // declaration includes only the related features part, then the keyword of can be
    // omitted" (7.4.6.3, receipt cda2047f).
    //
    // The node is built even when empty, as SuccessionDeclaration's is.
    fn binding_connector_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnectorDeclaration);
        let after_all = usize::from(self.at_keyword("all"));
        let empty = self.nth_is(after_all, SyntaxKind::Semicolon)
            || self.nth_is(after_all, SyntaxKind::LBrace)
            || self.peek_nth(after_all).is_none();
        let ends = self.nth_is_keyword(after_all, "of")
            || self
                .skip_connector_end(after_all)
                .is_some_and(|after| self.nth_is(after, SyntaxKind::Eq));
        if empty || ends {
            self.eat_optional_keyword("all");
            if ends {
                self.eat_optional_keyword("of");
                self.connector_end_member();
                self.expect(SyntaxKind::Eq, "`=`");
                self.connector_end_member();
            }
        } else {
            self.feature_declaration();
            if self.at_keyword("of") {
                self.bump_as(keyword("of").unwrap_or(SyntaxKind::BasicName));
                self.connector_end_member();
                self.expect(SyntaxKind::Eq, "`=`");
                self.connector_end_member();
            }
        }
        self.finish_node();
    }

    // production: FlowEnd@kerml
    //
    // FlowEnd = ( ownedRelationship += OwnedReferenceSubsetting '.' )?
    //           ownedRelationship += FlowFeatureMember           (KerML 8.2.5.9.2)
    //
    // KerML's states the `.` SysML's needs a deviation for, and has no FeatureChainPrefix:
    // its OwnedReferenceSubsetting is a name or an OwnedFeatureChain (GeneralType,
    // 8.2.4.3.3), so an end of three or more segments subsets the chain of all but the
    // last, `a.b.c` being `a.b`, `.`, `c`. The segments are counted first, as SysML's end
    // does, because a chain read greedily would take the last one too. So this end builds
    // its OwnedReferenceSubsetting itself, in `flow_end_subsetting`, where References
    // builds it through `owned_reference_subsetting`, which carries the @kerml marker: the
    // same node, text and tree, bounded here to stop before the end's last segment.
    pub(super) fn kerml_flow_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FlowEnd);
        match self.flow_end_segments(0) {
            None => self.error_expected("a flow end"),
            Some((_, 1)) => self.flow_feature_member(),
            Some((_, segments)) => {
                self.flow_end_subsetting(segments - 1);
                self.expect(SyntaxKind::Dot, "`.`");
                self.flow_feature_member();
            }
        }
        self.finish_node();
    }

    /// Which of `KerML`'s `Function` and `Predicate` starts at the `n`th meaningful token,
    /// if either does: `function` or `predicate`, reserved (`KerML` 8.2.2.6), after a
    /// `TypePrefix`. The keyword and the node it builds.
    pub(super) fn at_kerml_function(&self, n: usize) -> Option<(&'static str, SyntaxKind)> {
        let after = self.skip_type_prefix(n);
        [
            ("function", SyntaxKind::Function),
            ("predicate", SyntaxKind::Predicate),
        ]
        .into_iter()
        .find(|(word, _)| self.nth_is_keyword(after, word))
    }

    // production: Function@kerml
    // production: Predicate@kerml
    //
    // Function  = TypePrefix 'function'  ClassifierDeclaration FunctionBody
    //                                                            (KerML 8.2.5.7.1)
    // Predicate = TypePrefix 'predicate' ClassifierDeclaration FunctionBody
    //                                                            (KerML 8.2.5.7.3)
    //
    // The classifiers' spine with a FunctionBody where theirs has a TypeBody, so one
    // method reads both, as `classifier` reads the nine, and neither is in CLASSIFIERS.
    // A Function is a Behavior whose result is its evaluation (8.3.4.7.4, receipt
    // 0346a271), "declared as a behavior ..., using the keyword function" (7.4.8.2,
    // receipt 29d9c85d); a Predicate is a Function with a Boolean result (8.3.4.7.6,
    // receipt 38521fba), whose "body ... is the same as a function body" (7.4.8.4,
    // receipt 0dccac6d).
    //
    // implied specialization: Performances::Evaluation for a Function,
    //     Performances::BooleanEvaluation for a Predicate
    // constraint: Function::checkFunctionSpecialization (KerML 8.3.4.7.4) and
    //     Predicate::checkPredicateSpecialization (8.3.4.7.6). Injections, sv2-hir's;
    //     nothing is written into the tree. The result constraints of 8.3.4.7.4
    //     (validateFunctionResultExpressionMembership and its kin) are validity, not
    //     syntax (ADR-0002).
    pub(super) fn kerml_function(&mut self, (word, node): (&'static str, SyntaxKind)) {
        self.eat_trivia();
        self.start_node(node);
        self.type_prefix();
        self.expect_keyword(word);
        self.classifier_declaration();
        self.function_body();
        self.finish_node();
    }
}
