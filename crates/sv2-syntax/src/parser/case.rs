// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Cases, `SysML` 8.2.2.22 to 8.2.2.25: case bodies, objectives, analysis, verification
//! and use cases, and `include`.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;

/// A case production pair: one kind keyword run over the case layer's spine.
///
/// ```text
/// CaseDefinition         = OccurrenceDefinitionPrefix 'case' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.22
/// CaseUsage              = OccurrenceUsagePrefix 'case'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.22
/// AnalysisCaseDefinition = OccurrenceDefinitionPrefix 'analysis' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.23
/// AnalysisCaseUsage      = OccurrenceUsagePrefix 'analysis'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.23
/// VerificationCaseDefinition = OccurrenceDefinitionPrefix 'verification' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.24
/// VerificationCaseUsage  = OccurrenceUsagePrefix 'verification'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.24
/// UseCaseDefinition      = OccurrenceDefinitionPrefix 'use' 'case' 'def'
///                          DefinitionDeclaration CaseBody            SysML 8.2.2.25
/// UseCaseUsage           = OccurrenceUsagePrefix 'use' 'case'
///                          ConstraintUsageDeclaration CaseBody       SysML 8.2.2.25
/// ```
///
/// "An analysis case definition or usage is declared as a case definition or usage ...
/// using the kind keyword analysis" (7.23.2, receipt 2aa2d6ce), a verification case "using
/// the kind keyword verification" (7.24.2, receipt d518fc8c), and a use case "using the
/// kind keyword use case" (7.25.2, receipt 9be3712a), so the pairs differ in the keywords
/// and the metaclass alone. These four are every case definition and usage pair 8.2.2
/// states; `IncludeUseCaseUsage` (8.2.2.25) is a case usage too, but has no definition and
/// a declaration of its own, so it is read by `include_use_case_usage`.
#[derive(Clone, Copy)]
pub(super) struct Case {
    /// The kind keywords in order, before the `def` of a definition: one, or `use case`'s
    /// two. The runs are disjoint on their FIRST word — `case` alone never begins
    /// `use case` — which is what lets the recognisers match a row from its start.
    keywords: &'static [&'static str],
    /// The node the definition production builds.
    definition: SyntaxKind,
    /// The node the usage production builds.
    usage: SyntaxKind,
}

/// Every case production pair read. The first keywords are reserved and disjoint, so the
/// order decides nothing.
pub(super) const CASES: [Case; 4] = [
    Case {
        keywords: &["case"],
        definition: SyntaxKind::CaseDefinition,
        usage: SyntaxKind::CaseUsage,
    },
    Case {
        keywords: &["analysis"],
        definition: SyntaxKind::AnalysisCaseDefinition,
        usage: SyntaxKind::AnalysisCaseUsage,
    },
    Case {
        keywords: &["verification"],
        definition: SyntaxKind::VerificationCaseDefinition,
        usage: SyntaxKind::VerificationCaseUsage,
    },
    Case {
        keywords: &["use", "case"],
        definition: SyntaxKind::UseCaseDefinition,
        usage: SyntaxKind::UseCaseUsage,
    },
];

impl Parser<'_> {
    /// Which case definition starts at the `n`th meaningful token, if one does.
    ///
    /// `OccurrenceDefinitionPrefix`, a `CASES` keyword and `def` (`SysML` 8.2.2.22,
    /// 8.2.2.23). The prefix skipped is the one `case_definition` reads.
    pub(super) fn at_case_definition(&self, n: usize) -> Option<Case> {
        let after = self.skip_occurrence_definition_prefix(n);
        CASES.into_iter().find(|case| {
            self.case_keywords_at(after, case)
                .is_some_and(|k| self.nth_is_keyword(k, "def"))
        })
    }

    /// The index just past `case`'s kind keywords when all of them are written from the
    /// `n`th token, in order.
    fn case_keywords_at(&self, n: usize, case: &Case) -> Option<usize> {
        let mut k = n;
        for word in case.keywords {
            if !self.nth_is_keyword(k, word) {
                return None;
            }
            k += 1;
        }
        Some(k)
    }

    /// Which case usage starts at the `n`th meaningful token, if one does.
    ///
    /// `OccurrenceUsagePrefix` and a `CASES` keyword run with no `def` after it, which is
    /// what makes it the definition beside it. The prefix skipped is the one `case_usage`
    /// reads with `occurrence_usage_prefix`. A `use case` is never taken for a `case`: `use`
    /// is not in the prefix, so the `case` after it is never at the position asked.
    pub(super) fn at_case_usage(&self, n: usize) -> Option<Case> {
        let after = self.skip_occurrence_usage_prefix(n);
        CASES.into_iter().find(|case| {
            self.case_keywords_at(after, case)
                .is_some_and(|k| !self.nth_is_keyword(k, "def"))
        })
    }

    /// Whether an `IncludeUseCaseUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'include'` (`SysML` 8.2.2.25). Like `perform`, the keyword
    /// names a usage and nothing else, so there is no `def` to test. The prefix skipped is
    /// the one `include_use_case_usage` reads with `occurrence_usage_prefix`.
    pub(super) fn at_include_use_case_usage(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_occurrence_usage_prefix(n), "include")
    }

    // production: IncludeUseCaseUsage
    //
    // IncludeUseCaseUsage = OccurrenceUsagePrefix 'include'
    //     ( ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
    //     | 'use' 'case' UsageDeclaration )
    //     ValuePart? CaseBody                                    (SysML 8.2.2.25)
    //
    // PerformActionUsageDeclaration's two alternatives over a use case: "declared as a
    // use case usage ... using the kind keyword include use case", or "using just the
    // keyword include ... the included use case ... identified by giving a qualified name
    // or feature chain immediately after the include keyword" (7.25.3, receipt 6f1b9dfd).
    // Told apart on one token, as there: `use` is reserved and a reference opens on a
    // name. The body is a CaseBody because 8.2.2.25 writes one. The metaclass is
    // IncludeUseCaseUsage (8.3.25.2, receipt 40fbe5a7), both a UseCaseUsage and a
    // PerformActionUsage.
    //
    // The reference alternative's FeatureSpecializationPart may open on a multiplicity
    // (`include 'add fuel'[0..*] { }`, training/35. Use Cases/Use Case Usage Example.sysml
    // and validation/18-Use Case), so `[` is asked for as `usage_declaration` asks for it.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: UseCases::UseCase::includedUseCases, when owned by a use case
    // constraint: IncludeUseCaseUsage::checkIncludeUseCaseSpecialization (8.3.25.2). An
    //     injection, so sv2-hir's (ADR-0002).
    // constraint: IncludeUseCaseUsage::validateIncludeUseCaseUsageReference (8.3.25.2):
    //     the reference's target must be a UseCaseUsage. A question of resolution, so
    //     sv2-resolve's.
    pub(super) fn include_use_case_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::IncludeUseCaseUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("include");
        if self.at_keyword("use") {
            self.bump_as(keyword("use").unwrap_or(SyntaxKind::BasicName));
            self.expect_keyword("case");
            self.usage_declaration();
        } else {
            self.owned_reference_subsetting();
            self.optional_feature_specialization_part();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.case_body();
        self.finish_node();
    }

    // production: CaseDefinition
    // production: AnalysisCaseDefinition
    // production: VerificationCaseDefinition
    // production: UseCaseDefinition
    //
    // CaseDefinition         = OccurrenceDefinitionPrefix 'case' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.22)
    // AnalysisCaseDefinition = OccurrenceDefinitionPrefix 'analysis' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.23)
    // VerificationCaseDefinition = OccurrenceDefinitionPrefix 'verification' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.24)
    // UseCaseDefinition      = OccurrenceDefinitionPrefix 'use' 'case' 'def'
    //                          DefinitionDeclaration CaseBody            (SysML 8.2.2.25)
    //
    // One method, the keywords and node from `CASES`. "A case definition or usage is
    // declared as a kind of calculation definition or usage ... using the kind keyword
    // case" (7.22.2, receipt eb25a69f), and an analysis case as a case with the kind
    // keyword analysis (7.23.2, receipt 2aa2d6ce), a verification case with
    // `verification` (7.24.2, receipt d518fc8c), a use case with `use case` (7.25.2,
    // receipt 9be3712a): CalculationDefinition's spine over CaseBody. The metaclasses
    // chain AnalysisCaseDefinition, VerificationCaseDefinition and UseCaseDefinition >
    // CaseDefinition > CalculationDefinition (8.3.23.2, receipt 188d1035; 8.3.24.3,
    // receipt 0cc87426; 8.3.25.3, receipt 32b25e8d; 8.3.22.2, receipt 692a4982).
    //
    // implied specialization: Cases::Case, AnalysisCases::AnalysisCase,
    //     VerificationCases::VerificationCase or UseCases::UseCase
    // constraint: CaseDefinition::checkCaseDefinitionSpecialization,
    //     `specializesFromLibrary('Cases::Case')` (8.3.22.2), and
    //     AnalysisCaseDefinition::checkAnalysisCaseDefinitionSpecialization,
    //     `specializesFromLibrary('AnalysisCases::AnalysisCase')` (8.3.23.2), and
    //     VerificationCaseDefinition::checkVerificationCaseSpecialization,
    //     `specializesFromLibrary('VerificationCases::VerificationCase')` (8.3.24.3), and
    //     UseCaseDefinition::checkUseCaseDefinitionSpecialization,
    //     `specializesFromLibrary('UseCases::UseCase')` (8.3.25.3). Injections, so
    //     sv2-hir's; this layer builds the tree only (ADR-0002).
    // constraint: UseCaseDefinition::deriveUseCaseDefinitionIncludedUseCase (8.3.25.3), a
    //     derivation over the IncludeUseCaseUsages the body owns; sv2-resolve's.
    // constraint: CaseDefinition::validateCaseDefinitionOnlyOneSubject,
    //     validateCaseDefinitionOnlyOneObjective and
    //     validateCaseDefinitionSubjectParameterPosition (8.3.22.2): at most one subject
    //     and one objective, and the subject the first parameter. Constraints on what the
    //     body holds, not grammar, so sv2-resolve's; the body reads any number of each.
    pub(super) fn case_definition(&mut self, case: Case) {
        self.eat_trivia();
        self.start_node(case.definition);
        self.occurrence_definition_prefix();
        for word in case.keywords {
            self.expect_keyword(word);
        }
        self.expect_keyword("def");
        self.definition_declaration();
        self.case_body();
        self.finish_node();
    }

    // production: CaseUsage
    // production: AnalysisCaseUsage
    // production: VerificationCaseUsage
    // production: UseCaseUsage
    //
    // CaseUsage         = OccurrenceUsagePrefix 'case'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.22)
    // AnalysisCaseUsage = OccurrenceUsagePrefix 'analysis'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.23)
    // VerificationCaseUsage = OccurrenceUsagePrefix 'verification'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.24)
    // UseCaseUsage      = OccurrenceUsagePrefix 'use' 'case'
    //                     ConstraintUsageDeclaration CaseBody            (SysML 8.2.2.25)
    //
    // ConstraintUsageDeclaration and not ActionUsageDeclaration, although a CaseUsage is a
    // CalculationUsage (8.3.22.3, receipt f0ff6680) and CalculationUsage takes the
    // action's: the grammar states it so. The two bodies are the same,
    // `UsageDeclaration ValuePart?` (8.2.2.20, 8.2.2.17.2), so the choice decides the node
    // name alone and not what text is read.
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Cases::cases, AnalysisCases::analysisCases,
    //     VerificationCases::verificationCases or UseCases::useCases
    // constraint: CaseUsage::checkCaseUsageSpecialization (8.3.22.3),
    //     AnalysisCaseUsage::checkAnalysisCaseUsageSpecialization (8.3.23.3, receipt
    //     ac8c6d0a), VerificationCaseUsage::checkVerificationCaseUsageSpecialization
    //     (8.3.24.4, receipt 980c6c7a) and UseCaseUsage::checkUseCaseUsageSpecialization
    //     (8.3.25.4, receipt b7b869b1), and the composite-owned
    //     checkCaseUsageSubcaseSpecialization,
    //     checkAnalysisCaseUsageSubAnalysisCaseSpecialization,
    //     checkVerificationCaseUsageSubVerificationCaseSpecialization and
    //     checkUseCaseUsageSubUseCaseSpecialization. sv2-hir's (ADR-0002).
    pub(super) fn case_usage(&mut self, case: Case) {
        self.eat_trivia();
        self.start_node(case.usage);
        self.occurrence_usage_prefix();
        for word in case.keywords {
            self.expect_keyword(word);
        }
        self.constraint_usage_declaration();
        self.case_body();
        self.finish_node();
    }

    // production: CaseBody
    //
    // CaseBody : Type =
    //     ';'
    //   | '{' CaseBodyItem* ( ownedRelationship += ResultExpressionMember )? '}'
    //                                                            (SysML 8.2.2.22)
    //
    // CalculationBody's shape with the part inlined, so there is no CaseBodyPart node:
    // the grammar names none. The items are read by `body_elements` under `Body::Case`,
    // which says what they are; the trailing expression as `calculation_body_part` reads
    // its own.
    //
    // CaseBodyItem is marked at `body_element`:
    //
    //     CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember | ObjectiveMember
    //
    // all four read under Body::Case, and `return` too, by deviation CaseBodyItem; see
    // `Body::admits_return_parameter`.
    fn case_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::CaseBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Case);
            // As in `calculation_body_part`: the loop returned before the `}` only
            // because what is left is the trailing ResultExpressionMember.
            if !self.at_end() && !self.at(SyntaxKind::RBrace) {
                self.result_expression_member();
            }
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a case declaration");
        }
        self.finish_node();
    }

    // production: ObjectiveMember
    //
    // ObjectiveMember : ObjectiveMembership =
    //     MemberPrefix 'objective'
    //     ownedRelatedElement += ObjectiveRequirementUsage       (SysML 8.2.2.22)
    //
    // "The objective of a case definition or usage is declared as a requirement usage
    // ..., but using the keyword objective instead of requirement" (7.22.2, receipt
    // eb25a69f). Unlike SubjectMember, the keyword belongs to the MEMBER here, not to the
    // usage it owns. The metaclass is ObjectiveMembership (8.3.22.4, receipt d68190c3), a
    // FeatureMembership.
    //
    // constraint: ObjectiveMembership::validateObjectiveMembershipOwningType (8.3.22.4):
    //     `owningType.oclIsType(CaseDefinition) or owningType.oclIsType(CaseUsage)`. The
    //     grammar says only part of it: CaseBodyItem alone reaches this, but the analysis,
    //     verification and use case bodies reach CaseBodyItem too, and the corpus writes
    //     objectives there (AnalysisTest.sysml, Annex A's use cases), which an exact-type
    //     test refuses. sv2-resolve's to decide, as a spec question and not this layer's.
    // constraint: ObjectiveMembership::validateObjectiveMembershipIsComposite (8.3.22.4).
    //     sv2-resolve's; the tree carries no `ref` here to contradict it.
    pub(super) fn objective_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ObjectiveMember);
        self.member_prefix();
        self.expect_keyword("objective");
        self.objective_requirement_usage();
        self.finish_node();
    }

    // production: ObjectiveRequirementUsage@sysml
    //
    // ObjectiveRequirementUsage : RequirementUsage =
    //     UsageExtensionKeyword* ConstraintUsageDeclaration RequirementBody
    //                                                            (SysML 8.2.2.22)
    //
    // `objective #goal o;` carries one. A RequirementBody, so a `require` or
    // `subject` inside an objective reads as it does inside a requirement; "the subject of
    // an objective requirement is bound by default to the result" (7.22.2) is a binding
    // sv2-hir injects, not text.
    fn objective_requirement_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ObjectiveRequirementUsage);
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.constraint_usage_declaration();
        self.requirement_body();
        self.finish_node();
    }
}
