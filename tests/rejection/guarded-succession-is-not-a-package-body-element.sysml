// REJECTED BY RULE. A package body has no ActionBodyItem: PackageBodyElement is
// PackageMember, ElementFilterMember, AliasMember or Import, so no succession of any form
// is an element there.
// SysML 8.2.2.5.1   — PackageBodyElement = PackageMember | ElementFilterMember
//                                        | AliasMember | Import
// SysML 8.2.2.17.1  — ActionBodyItem = ... | GuardedSuccessionMember
package P {
	first a if x then b;
}
