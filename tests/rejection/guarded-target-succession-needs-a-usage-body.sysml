// REJECTED BY RULE. ActionTargetSuccession ends in UsageBody, which is `;` or a braced
// body, whichever of the three successions it took. The two forms are rejected at
// different points, and only the plain one at recognition: `at_target_succession` requires
// the UsageBody's opening token before it commits, while the guarded form is recognised by
// its `then` alone -- so the member IS read here and the missing body is reported by
// UsageBody itself. Both are errors; the tree carries the member either way (ADR-0002).
// SysML 8.2.2.17.8  — ActionTargetSuccession = ( TargetSuccession
//                                             | GuardedTargetSuccession
//                                             | DefaultTargetSuccession ) UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
// rejects: PARSE-EXPECTED on: }
action def A {
	first start;
	if x then b
}
