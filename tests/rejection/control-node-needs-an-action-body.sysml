// REJECTED BY RULE. Each control node ends in an ActionBody, which is `;` or a braced
// body -- not optional.
// SysML 8.2.2.17.3  — JoinNode = ControlNodePrefix 'join' UsageDeclaration ActionBody
// SysML 8.2.2.17.1  — ActionBody = ';' | '{' ActionBodyItem* '}'
action def A {
	join j
}
