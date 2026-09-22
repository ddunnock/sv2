// REJECTED BY RULE. A dependency is a DefinitionElement, not an occurrence usage, so no
// SourceSuccessionMember prefixes it.
// SysML 8.2.2.5.2 — DefinitionElement = Package | LibraryPackage | AnnotatingElement
//                   | Dependency | …
// SysML 8.2.2.6.1 — DefinitionBodyItem = DefinitionMember | …
//                   | SourceSuccessionMember? OccurrenceUsageMember | …
// rejects: PARSE-UNEXPECTED on: then dependency a to b;
part def P {
	then dependency a to b;
}
