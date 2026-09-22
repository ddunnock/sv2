// REJECTED BY RULE. An enumerated value is `'enum'? Usage`: no UsagePrefix, so no
// direction, `derived`, `abstract` or the rest. "The declaration of an enumeration usage
// as an enumerated value of an enumeration definition may not include the keyword variant
// nor any of the other property keywords given in 7.6.3 (i.e, any direction keywords,
// abstract, derived, etc.)" (7.8.2, receipt a2406cd5).
// SysML 8.2.2.8 — EnumerationUsageMember = MemberPrefix EnumeratedValue
//                 EnumeratedValue = 'enum'? Usage
// rejects: PARSE-UNEXPECTED on: in a;
enum def E {
	in a;
}
