// REJECTED BY RULE. `part def` is a SysML DefinitionElement, reachable from SysML's
// start symbol and from no KerML production. KerML and SysML are two grammars over a
// shared vocabulary, not one that extends the other (ADR-0014), so a SysML construct
// in a .kerml file is text KerML does not admit.
// SysML 8.2.2.5.1 — RootNamespace = PackageBodyElement*
//                   PackageBodyElement = PackageMember | ElementFilterMember
//                                      | AliasMember | Import
//                   PackageMember = MemberPrefix ( DefinitionElement | UsageElement )
// KerML 8.2.3.4.1 — RootNamespace = NamespaceBodyElement*
//                   NamespaceBodyElement = NamespaceMember | AliasMember | Import
//                   NamespaceMember = NonFeatureMember | NamespaceFeatureMember
//                   NonFeatureMember = MemberPrefix MemberElement
//                   MemberElement = AnnotatingElement | NonFeatureElement
// Neither MemberElement nor FeatureElement reaches DefinitionElement.
//
// The same line in a .sysml file parses, which is what makes this a rule and not an
// absence — and the reason it is worth a case at all: before ADR-0014 was implemented
// in the parser, one root was applied to both file kinds and this was ACCEPTED.
part def Vehicle;
