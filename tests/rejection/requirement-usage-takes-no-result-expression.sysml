// REJECTED BY RULE. A requirement is a kind of constraint (7.21.2), but its body is a
// RequirementBody, not a CalculationBody: `RequirementBodyItem*` and nothing after them.
// Its constraint is built from the assumed and required constraints in the body, not
// written as a trailing result expression, so an expression there is no item. Written
// parenthesised so that nothing but an expression can start there.
// SysML 8.2.2.21.2 — RequirementUsage = OccurrenceUsagePrefix 'requirement'
//                                       ConstraintUsageDeclaration RequirementBody
// SysML 8.2.2.21.1 — RequirementBody = ';' | '{' RequirementBodyItem* '}'
// SysML 8.2.2.20   — ConstraintUsage = ... ConstraintUsageDeclaration CalculationBody
// rejects: PARSE-UNEXPECTED on: (a <= b)
part def P {
	requirement r {
		(a <= b)
	}
}
