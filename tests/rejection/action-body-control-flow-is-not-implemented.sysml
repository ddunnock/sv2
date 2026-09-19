// REJECTED BY ABSENCE, not by rule. Every line below is well-formed SysML and the
// pinned corpus writes this text constantly — 403 `then` and 139 `first`.
// SysML 8.2.2.17.1 — ActionBodyItem = NonBehaviorBodyItem
//                                   | ownedRelationship += InitialNodeMember
//                                     ( ownedRelationship
//                                         += ActionTargetSuccessionMember )*
//                                   | ( ownedRelationship += SourceSuccessionMember )?
//                                     ownedRelationship += ActionBehaviorMember
//                                     ( ownedRelationship
//                                         += ActionTargetSuccessionMember )*
//                                   | ownedRelationship += GuardedSuccessionMember
// ActionBody IS implemented and reads NonBehaviorBodyItem's implemented members, the whole
// of the second alternative (InitialNodeMember and, in its TargetSuccession form, the
// ActionTargetSuccessionMember* after it), and the third alternative over the behaviour
// usages this parser has, ActionUsage and PerformActionUsage -- and, from the next commit,
// over the four ControlNodes (8.2.2.17.3). So every line below parses but the last:
// `then accept s;` is SourceSuccessionMember followed by an ActionNodeMember over an
// AcceptNode (8.2.2.17.4), which is not implemented. It, the other non-control
// ActionNodes, GuardedSuccessionMember, and the guarded and default target successions
// are what is left of the control-flow layer.
// History: this file rejected on `then accelerate;` until ActionTargetSuccessionMember
// landed, on `then action decelerate;` until the third alternative did, and on
// `then merge m;` until ControlNode did. Every line is kept, and each is asserted
// ACCEPTED in crates/sv2-syntax/tests/parser.rs.
// Measured when ActionBody landed: of the 46 corpus files that write an `action def` and
// still fail, exactly ONE fails first on `first`. The rest fail on ActionUsage and
// PerformActionUsage, which are declarations rather than control flow.
action def Brake {
	first start;
	then accelerate;
	then action decelerate;
	then merge m;
	then accept s;
}
