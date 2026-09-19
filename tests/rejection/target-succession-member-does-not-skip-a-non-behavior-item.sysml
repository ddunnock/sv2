// REJECTED BY RULE. The member must follow its InitialNodeMember DIRECTLY; an attribute
// between them is a NonBehaviorBodyItem, which takes no ActionTargetSuccessionMember
// suffix. SysML 7.17.4 says the source of a `then` is "the nearest occurrence lexically
// previous to the then, skipping over any" intervening non-occurrence usages — that
// decides which element is the source, for resolution, not which text parses.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember* | …
// SysML 7.17.4      — wiki receipt 339ef468 (Succession-Shorthands)
action def A {
	first start;
	attribute x;
	then a;
}
