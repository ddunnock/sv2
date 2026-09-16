// Rule: DefinitionElement has 28 alternatives and this parser implements one.
// SysML 8.2.2.5.2 — DefinitionElement = Package | LibraryPackage | ... | PartDefinition | ...
// `part def` is a PartDefinition, which is unimplemented, so it must be reported
// rather than accepted behind a visibility that did parse.
// Remove this case when PartDefinition lands; it is a rejection by absence, not by
// rule, and the header says so on purpose.
public part def Vehicle;
