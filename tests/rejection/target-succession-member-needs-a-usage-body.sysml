// REJECTED BY RULE. ActionTargetSuccession ends in UsageBody, which is `;` or a braced
// body; `then a` followed by neither is no production. The parser declines it at
// recognition — the lookahead requires the UsageBody's opening token before it commits —
// so the `then` is recovered over and reported rather than read as a member missing its
// body.
// SysML 8.2.2.17.8  — ActionTargetSuccession = ( TargetSuccession | … ) UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
action def A {
	first start;
	then a
}
