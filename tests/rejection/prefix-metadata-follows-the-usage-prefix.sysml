// REJECTED BY RULE. A user-defined keyword follows a usage's direction, `derived`,
// `abstract`, `constant` and `ref`; it does not precede them. "The user-defined keyword is
// placed immediately before the language-defined (reserved) keyword for the declaration"
// (7.27.4, receipt 0f2c5bd1).
// SysML 8.2.2.6.2 — UsagePrefix = UnextendedUsagePrefix UsageExtensionKeyword*
//                   UnextendedUsagePrefix = EndUsagePrefix | BasicUsagePrefix
// rejects: PARSE-UNEXPECTED on: #Approved in attribute mass : Real;
part def Vehicle {
	#Approved in attribute mass : Real;
}
