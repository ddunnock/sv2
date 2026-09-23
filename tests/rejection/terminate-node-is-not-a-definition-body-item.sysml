// REJECTED BY RULE. A TerminateNode is an ActionNode, owned through ActionNodeMember,
// which only ActionBehaviorMember reaches (8.2.2.17.1): an item of an action body, not of
// a part definition's DefinitionBody (8.2.2.6.1), whose occurrence usages are
// StructureUsageElements and BehaviorUsageElements and never ActionNodes (8.2.2.6.4).
// SysML 8.2.2.17.1 — ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember
// SysML 8.2.2.17.2 — ActionNode = ControlNode | SendNode | AcceptNode | AssignmentNode
//                               | TerminateNode | IfNode | WhileLoopNode | ForLoopNode
// rejects: PARSE-UNEXPECTED on: terminate;
part def Processor {
	terminate;
}
