// REJECTED BY RULE. A namespace's body is NamespaceBodyElements: members, aliases and
// imports. An ElementFilterMember is a package body's element alone.
// KerML 8.2.3.4.1 — NamespaceBody        = ';' | '{' NamespaceBodyElement* '}'
//                   NamespaceBodyElement = NamespaceMember | AliasMember | Import
// KerML 8.2.5.13  — PackageBody = ';' | '{' ( NamespaceBodyElement
//                                           | ElementFilterMember )* '}'
// rejects: PARSE-UNEXPECTED on: filter true;
namespace N2 {
	class C;
	filter true;
}
