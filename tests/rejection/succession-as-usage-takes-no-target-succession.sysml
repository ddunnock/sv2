// REJECTED BY RULE. In an action body a SuccessionAsUsage is a NonOccurrenceUsageElement,
// read through ActionBodyItem's FIRST alternative, NonBehaviorBodyItem, which carries no
// ActionTargetSuccessionMember* suffix; the second and third alternatives carry one. So
// `then c;` after a succession is no item. A positive corpus cannot show the asymmetry.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// SysML 8.2.2.6.4   — NonOccurrenceUsageElement = ... | SuccessionAsUsage | ...
// rejects: PARSE-UNEXPECTED on: then c;
action def A {
	first a then b;
	then c;
}
