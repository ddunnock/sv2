// REJECTED BY RULE. In an action body a BindingConnectorAsUsage is a
// NonOccurrenceUsageElement, read through ActionBodyItem's FIRST alternative,
// NonBehaviorBodyItem, which carries no ActionTargetSuccessionMember* suffix. So `then c;`
// after a binding is no item. A positive corpus cannot show the asymmetry.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// SysML 8.2.2.6.4   — NonOccurrenceUsageElement = ... | BindingConnectorAsUsage | ...
// rejects: PARSE-UNEXPECTED on: then c;
action def A {
	bind a = b;
	then c;
}
