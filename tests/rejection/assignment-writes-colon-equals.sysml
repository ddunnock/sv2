// REJECTED BY RULE. An assignment writes `:=` between its referent and its value, not
// `=`: "followed by the symbol := and an expression whose result is the assigned value"
// (7.17.9, receipt 7d690ecc).
// SysML 8.2.2.17.5 — AssignmentNodeDeclaration = ActionNodeUsageDeclaration? 'assign'
//                    AssignmentTargetMember FeatureChainMember ':=' NodeParameterMember
// rejects: PARSE-EXPECTED on: assign x = 1;
action def A {
	assign x = 1;
}
