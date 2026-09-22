// REJECTED BY RULE. A transition usage is an item of a state body alone: StateBodyItem
// names TransitionUsageMember, and ActionBodyItem, DefinitionBodyItem and the rest do not.
// SysML 8.2.2.18.1 — StateBodyItem = NonBehaviorBodyItem | … | TransitionUsageMember | …
// SysML 8.2.2.17.1 — ActionBodyItem has no TransitionUsageMember alternative.
// rejects: PARSE-UNEXPECTED on: transition first a then b;
action def A {
	transition first a then b;
}
