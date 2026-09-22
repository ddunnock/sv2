// REJECTED BY RULE. An enumeration definition's values are its variants (8.4.4), but they
// are written as enumerated values, and "the declaration of an enumeration usage as an
// enumerated value of an enumeration definition may not include the keyword variant"
// (7.8.2, receipt a2406cd5).
// SysML 8.2.2.8 — EnumerationBody = ';'
//                   | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'
// rejects: PARSE-UNEXPECTED on: variant a;
enum def E {
	variant a;
}
