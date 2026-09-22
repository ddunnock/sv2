// REJECTED BY RULE. EnumerationDefinition opens on DefinitionExtensionKeyword*, not on a
// DefinitionPrefix, so `abstract` and `variation` have no place in it. "The keywords
// abstract and variation may not be used with an enumeration definition" (7.8.2, receipt
// a2406cd5): an enumeration definition is always a variation already.
// SysML 8.2.2.8   — EnumerationDefinition = DefinitionExtensionKeyword* 'enum' 'def' ...
// SysML 8.2.2.6.1 — DefinitionPrefix = BasicDefinitionPrefix? DefinitionExtensionKeyword*
// rejects: PARSE-UNEXPECTED on: abstract enum def E;
abstract enum def E;
