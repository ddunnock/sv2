// Rule: ';' on its own is not a PackageBodyElement.
// SysML 8.2.2.5.1 — PackageBodyElement =
//   PackageMember | ElementFilterMember | AliasMember | Import
// The ';' that closes a PackageBody belongs to the body, not to the namespace.
package Vehicle;;