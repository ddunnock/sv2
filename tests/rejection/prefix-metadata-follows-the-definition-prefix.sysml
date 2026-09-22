// REJECTED BY RULE. A user-defined keyword follows a definition's `abstract` or
// `variation` and its `individual`; it does not precede them. "The user-defined keyword is
// placed immediately before the language-defined (reserved) keyword for the declaration"
// (7.27.4, receipt 0f2c5bd1), and the grammar writes the prefix keywords first.
// SysML 8.2.2.9.1 — OccurrenceDefinitionPrefix = BasicDefinitionPrefix?
//   ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
//   DefinitionExtensionKeyword*
// rejects: PARSE-UNEXPECTED on: #Safety abstract part def Brake;
#Safety abstract part def Brake;
