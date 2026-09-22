// REJECTED BY RULE. An assignment is an ActionNode, and an ActionNode is an item of the
// action-body family only: ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember is
// reached from ActionBodyItem, and DefinitionBodyItem reaches no ActionNodeMember. "Every
// assignment action usage must be one of the following: 1. An owned feature of an action
// definition or usage. 2. The owned entry, do or exit action of a state ... 3. The owned
// effect of a transition usage" (7.17.9, receipt 7d690ecc).
// SysML 8.2.2.17.1 — ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember
// rejects: PARSE-UNEXPECTED on: assign x := 1;
part def P {
	assign x := 1;
}
