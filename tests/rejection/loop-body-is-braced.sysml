// REJECTED BY RULE. A loop's body clause "is itself notated as an action usage, but with
// its body required to be given using curly braces { … }, with a semicolon not allowed
// for an empty body" (7.17.12, receipt b0446148).
// SysML 8.2.2.17.7 — ActionBodyParameter = ( 'action' UsageDeclaration? )?
//                                         '{' ActionBodyItem* '}'
// rejects: PARSE-EXPECTED on: while not ready;
action def Poll {
	while not ready;
}
