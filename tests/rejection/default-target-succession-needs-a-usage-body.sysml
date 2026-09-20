// REJECTED BY RULE. ActionTargetSuccession ends in UsageBody -- `;` or a braced body --
// whichever of the three successions it took. Like the guarded form and unlike the plain
// one, this is recognised on its keyword alone, so the member IS read and UsageBody itself
// reports the missing body; the tree still carries the member (ADR-0002).
// SysML 8.2.2.17.8  — ActionTargetSuccession = ( TargetSuccession
//                                             | GuardedTargetSuccession
//                                             | DefaultTargetSuccession ) UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
// rejects: PARSE-EXPECTED on: }
action def A {
	first start;
	else b
}
