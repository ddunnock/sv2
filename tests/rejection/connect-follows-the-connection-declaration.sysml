// REJECTED BY RULE. The declaration comes after `connection` and before `connect`: "placed
// after the connection usage declaration and before its body" (7.13.2, receipt 5a3a8867).
// `connect` opens the shorthand with NO declaration, so what follows it is a connector
// part, and `c : T` is not one.
// SysML 8.2.2.13.1 — ConnectionUsage = OccurrenceUsagePrefix
//                    ( 'connection' UsageDeclaration ValuePart? ( 'connect' ConnectorPart )?
//                    | 'connect' ConnectorPart ) UsageBody
// rejects: PARSE-EXPECTED on: connect c : T connection a to b;
part def P {
	connect c : T connection a to b;
}
