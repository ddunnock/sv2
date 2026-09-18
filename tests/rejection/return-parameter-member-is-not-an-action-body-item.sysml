// REJECTED BY RULE. ActionBodyItem is CalculationBodyItem's OTHER alternative, not a
// superset of it, so an action body does not admit the `return` a calculation body does.
// SysML 8.2.2.19   — CalculationBodyItem = ActionBodyItem | ReturnParameterMember
// SysML 8.2.2.17.1 — ActionBodyItem      = NonBehaviorBodyItem
//                                        | InitialNodeMember ActionTargetSuccessionMember*
//                                        | SourceSuccessionMember? ActionBehaviorMember
//                                          ActionTargetSuccessionMember*
//                                        | GuardedSuccessionMember
// This is the direction the containment actually runs, and it is easy to write the
// dispatch the other way round: Body::Action and Body::Calculation share their loop, and
// a return admitted by the loop rather than by the variant would be accepted here.
action def A {
	return x;
}
