// REJECTED BY RULE. The kind keyword of a use case is the two-word `use case`: "declared
// as a case definition or usage ..., using the kind keyword use case" (7.25.2, receipt
// 9be3712a). `use` alone opens no production, and `use def` is neither a use case
// definition nor anything else.
// SysML 8.2.2.25 — UseCaseDefinition = OccurrenceDefinitionPrefix 'use' 'case' 'def'
//                                      DefinitionDeclaration CaseBody
// rejects: PARSE-UNEXPECTED on: use def 'Provide Transportation';
package P {
	use def 'Provide Transportation';
}
