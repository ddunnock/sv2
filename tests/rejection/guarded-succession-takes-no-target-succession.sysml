// REJECTED BY RULE. The second and third ActionBodyItem alternatives carry an
// ActionTargetSuccessionMember* suffix and the fourth does not, so `then c;` following a
// guarded succession is no item. (NonBehaviorBodyItem, the first, carries no suffix
// either; what this file holds is that a SUCCESSION takes none.) The asymmetry would be
// invisible to a positive corpus, which never writes the form.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// rejects: PARSE-UNEXPECTED on: then c;
action def A {
	first a if x then b;
	then c;
}
