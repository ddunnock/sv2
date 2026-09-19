// REJECTED BY RULE. A package body has no SourceSuccessionMember: PackageBodyElement is
// PackageMember, ElementFilterMember, AliasMember or Import. `then` prefixes usages only in
// definition, requirement, action and calculation bodies.
// SysML 8.2.2.5.1   — PackageBodyElement = PackageMember | ElementFilterMember
//                                        | AliasMember | Import
package P {
	then part p;
}
