// REJECTED BY RULE. A package body has no ActionBodyItem: PackageBodyElement is
// PackageMember, ElementFilterMember, AliasMember or Import, so GuardedSuccession, which
// only ActionBodyItem reaches, is not an element there. (A SuccessionAsUsage IS one, as a
// UsageElement through PackageMember, 8.2.2.5.1-.2 and 8.2.2.6.4 — but it writes no guard.)
// SysML 8.2.2.5.1   — PackageBodyElement = PackageMember | ElementFilterMember
//                                        | AliasMember | Import
// SysML 8.2.2.17.1  — ActionBodyItem = ... | GuardedSuccessionMember
// rejects: PARSE-UNEXPECTED on: first a if x then b;
package P {
	first a if x then b;
}
