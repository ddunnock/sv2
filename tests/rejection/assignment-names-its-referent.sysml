// REJECTED BY RULE. The target of an assignment may be omitted and its referent may not:
// AssignmentTargetParameter is `( AssignmentTargetBinding '.' )?`, and the FeatureChainMember
// beside it has no `?`. "If the target expression of an assignment action usage is
// omitted, then the target is implicitly the occurrence owning the assignment action usage"
// (7.17.9, receipt 7d690ecc) -- the target, not the referent.
// SysML 8.2.2.17.5 — AssignmentNodeDeclaration = ActionNodeUsageDeclaration? 'assign'
//                    AssignmentTargetMember FeatureChainMember ':=' NodeParameterMember
// rejects: PARSE-EXPECTED on: assign := 1;
action def A {
	assign := 1;
}
