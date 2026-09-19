// REJECTED BY RULE. A package owns no control node: PackageBodyElement is PackageMember,
// ElementFilterMember, AliasMember or Import, and UsageElement has no ActionNode.
// SysML 8.2.2.5.1   — PackageBodyElement = PackageMember | ElementFilterMember
//                                        | AliasMember | Import
// SysML 8.3.17.6    — validateControlNodeOwningType (receipt 695df335)
package P {
	fork f;
}
