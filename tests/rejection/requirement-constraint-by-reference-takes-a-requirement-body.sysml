// REJECTED BY RULE, and this case IS the adjudicated conflict. It is the one file that
// fails if the parser ever follows the Pilot here instead of the specification.
// SysML 8.2.2.21.1 — RequirementConstraintUsage =
//       ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
//       RequirementBody
//     | ( UsageExtensionKeyword* 'constraint' | UsageExtensionKeyword+ )
//       ConstraintUsageDeclaration CalculationBody
// The clause gives the BY-REFERENCE alternative a RequirementBody. SysML.xtext gives it
// a CalculationBody. deviations.json records RequirementConstraintUsage as a conflict
// resolved follow_spec, adjudicated by the grammar-adjudicator on 2026-09-17.
// The difference is not cosmetic. Only a CalculationBody ends in a ResultExpressionMember
// (8.2.2.19), so a trailing expression with no semicolon is grammatical after
// `require constraint` and NOT after `require <name>`. This file writes the second.
// Under the Pilot's reading it would parse; under the specification's it must not.
// `require rangeRequirement { :>> actualRange = simulatedRange; }` — a redefinition, which
// a RequirementBody does admit — parses, and that case is in
// crates/sv2-syntax/tests/parser.rs.
requirement def R {
	require someRef { a <= b }
}
