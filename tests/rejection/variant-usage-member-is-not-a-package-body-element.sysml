// REJECTED BY RULE. A variant is an item of a definition, usage, requirement, action or
// calculation body, and not of a package body or the root.
// SysML 8.2.2.6.1  — DefinitionBodyItem = DefinitionMember | VariantUsageMember | …
// SysML 8.2.2.17.1 — NonBehaviorBodyItem = … | VariantUsageMember | …
// SysML 8.2.2.5.1  — PackageBodyElement = PackageMember | ElementFilterMember
//                                       | AliasMember | Import
// rejects: PARSE-UNEXPECTED on: variant part p;
package P {
	variant part p;
}
