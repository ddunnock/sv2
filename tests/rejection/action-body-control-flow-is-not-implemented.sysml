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
// ActionBody IS implemented and reads the part of NonBehaviorBodyItem this parser already
// had — Import, AliasMember and DefinitionMember — and the whole of the second
// alternative: InitialNodeMember and, in its TargetSuccession form, the
// ActionTargetSuccessionMember* after it. So `first start;` and `then accelerate;` below
// PARSE, and the rejection is on the last line: `then action …` is SourceSuccessionMember
// followed by ActionBehaviorMember, the third alternative, and both are unimplemented,
// as is the fourth. Together they are what is left of the control-flow layer.
// This file rejected on `then accelerate;` until ActionTargetSuccessionMember landed; that
// line is now asserted ACCEPTED in crates/sv2-syntax/tests/parser.rs.
// Measured when ActionBody landed: of the 46 corpus files that write an `action def` and
// still fail, exactly ONE fails first on `first`. The rest fail on ActionUsage and
// PerformActionUsage, which are declarations rather than control flow.
action def Brake {
	first start;
	then accelerate;
	then action decelerate;
}
