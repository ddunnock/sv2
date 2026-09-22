// REJECTED BY RULE. RequirementUsage ends in RequirementBody -- `;` or a braced body -- and
// it is the last part of the production, not an optional one. The usage is recognised by
// `requirement` with no `def` after it, so it IS read and RequirementBody reports its
// absence.
// SysML 8.2.2.21.2 — RequirementUsage = OccurrenceUsagePrefix 'requirement'
//                                       ConstraintUsageDeclaration RequirementBody
// SysML 8.2.2.21.1 — RequirementBody = ';' | '{' RequirementBodyItem* '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	requirement r
}
