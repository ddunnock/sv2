// REJECTED BY RULE. RequirementBodyItem reaches DefinitionBodyItem, not ActionBodyItem,
// so a requirement body owns no ActionNodeMember and no control node.
// SysML 8.2.2.21.1  — RequirementBodyItem = DefinitionBodyItem | SubjectMember | ...
// SysML 8.2.2.17.1  — ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember
requirement def R {
	decide d;
}
