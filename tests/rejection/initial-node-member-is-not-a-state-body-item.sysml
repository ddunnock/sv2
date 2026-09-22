// REJECTED BY RULE. `first X;` is ActionBodyItem's InitialNodeMember, and a state body
// reaches NonBehaviorBodyItem, not ActionBodyItem: a state's initial state is written as a
// succession from its entry action (7.18.2).
// SysML 8.2.2.18.1 — StateBodyItem = NonBehaviorBodyItem | … — no InitialNodeMember.
// SysML 8.2.2.17.1 — ActionBodyItem = … | InitialNodeMember ActionTargetSuccessionMember* | …
// rejects: PARSE-UNEXPECTED on: first start;
state def D {
	first start;
}
