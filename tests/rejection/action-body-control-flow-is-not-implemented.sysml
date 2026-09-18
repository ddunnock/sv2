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
// ActionBody IS implemented and reads the FIRST alternative only, and only the part of
// NonBehaviorBodyItem this parser already had: Import, AliasMember and DefinitionMember.
// The other three alternatives are the control-flow layer — initial nodes, successions
// and guards — and they are the largest unimplemented thing left in this grammar.
// Measured when ActionBody landed: of the 46 corpus files that write an `action def` and
// still fail, exactly ONE fails first on `first`. The rest fail on ActionUsage and
// PerformActionUsage, which are declarations rather than control flow.
action def Brake {
	first start;
	then accelerate;
}
