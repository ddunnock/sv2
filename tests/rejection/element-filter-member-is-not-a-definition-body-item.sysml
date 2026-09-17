// REJECTED BY RULE. ElementFilterMember is a PackageBodyElement and is NOT one of
// the six DefinitionBodyItem alternatives, so `filter` is well formed in a package
// body and not in a definition body. The same line inside `package P { ... }`
// parses, which is what makes this a rule and not an absence.
// SysML 8.2.2.5.1 — PackageBodyElement = PackageMember | ElementFilterMember
//                                      | AliasMember | Import
// SysML 8.2.2.6.1 — DefinitionBodyItem = DefinitionMember | VariantUsageMember
//                     | NonOccurrenceUsageMember
//                     | SourceSuccessionMember? OccurrenceUsageMember
//                     | AliasMember | Import
part def V { filter @Safety; }
