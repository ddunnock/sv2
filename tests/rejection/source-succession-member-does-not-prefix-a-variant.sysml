// REJECTED BY RULE. `then` prefixes an occurrence usage member in a definition body, and
// a variant is owned through a VariantUsageMember, which DefinitionBodyItem states as an
// alternative of its own with no SourceSuccessionMember before it.
// SysML 8.2.2.6.1 — DefinitionBodyItem = DefinitionMember | VariantUsageMember
//                   | NonOccurrenceUsageMember
//                   | SourceSuccessionMember? OccurrenceUsageMember | AliasMember | Import
// rejects: PARSE-UNEXPECTED on: then variant part p;
variation part def P {
	then variant part p;
}
