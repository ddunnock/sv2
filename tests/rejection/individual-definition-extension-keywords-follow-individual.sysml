// REJECTED BY RULE. A user-defined keyword follows `individual`, as it follows every
// other prefix keyword of a definition; it does not precede it. "The user-defined keyword
// is placed immediately before the language-defined (reserved) keyword for the
// declaration" (7.27.4, receipt 0f2c5bd1).
// SysML 8.2.2.9.1 — IndividualDefinition = BasicDefinitionPrefix? 'individual'
//                     DefinitionExtensionKeyword* 'def' Definition EmptyMultiplicityMember
// rejects: PARSE-UNEXPECTED on: #Test individual def Flight_248;
#Test individual def Flight_248;
