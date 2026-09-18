// REJECTED BY ABSENCE. The second alternative of RequirementConstraintUsage may be
// introduced by prefix metadata INSTEAD of the `constraint` keyword, and prefix metadata
// is unimplemented everywhere in this parser.
// SysML 8.2.2.21.1 — RequirementConstraintUsage = ...
//                      | ( UsageExtensionKeyword* 'constraint'
//                        | UsageExtensionKeyword+ )
//                        ConstraintUsageDeclaration CalculationBody
// SysML 8.2.2.6.2  — UsageExtensionKeyword = ownedRelationship += PrefixMetadataMember
// This is why RequirementConstraintUsage is not marked for coverage: the `*` form with
// zero keywords is read and is every instance the corpus writes, and the `+` form is not
// reachable at all. Removing this rejection means implementing PrefixMetadataMember.
requirement def R {
	require #approved { a <= b }
}
