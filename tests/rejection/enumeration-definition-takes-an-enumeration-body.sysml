// REJECTED BY ABSENCE, not by rule. EnumerationDefinition is the one `def`
// production that takes no prefix production at all — it opens on
// DefinitionExtensionKeyword*, not on a DefinitionPrefix — and it ends in an
// EnumerationBody rather than a DefinitionBody, so it is not on the shared spine at
// either end.
// SysML 8.2.2.8 — EnumerationDefinition = DefinitionExtensionKeyword* 'enum' 'def'
//                                         DefinitionDeclaration EnumerationBody
//                 EnumerationBody = ';'
//                                 | '{' ( AnnotatingMember
//                                       | EnumerationUsageMember )* '}'
// EnumerationBody is also the only unit that references AnnotatingMember, which is
// why AnnotatingMember is not built either: it would have no reachable caller.
enum def Color;
