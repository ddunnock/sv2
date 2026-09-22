// REJECTED BY RULE. "Any owned members declared in the body of an enumeration definition
// must be enumeration usages, which are the enumerated values of the enumeration
// definition" (7.8.2, receipt a2406cd5). EnumerationBody's items are AnnotatingMember and
// EnumerationUsageMember and nothing else, so a part usage is no item of it, where in an
// attribute definition's DefinitionBody it would be.
// SysML 8.2.2.8 — EnumerationBody = ';' | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'
// rejects: PARSE-UNEXPECTED on: part p;
enum def E {
	part p;
}
