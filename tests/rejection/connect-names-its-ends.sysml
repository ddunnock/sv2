// REJECTED BY RULE. `connect` takes a ConnectorPart, and both of its alternatives name
// ends: the connector part is optional after a `connection` declaration, never after the
// `connect` keyword itself.
// SysML 8.2.2.13.1 — ConnectionUsage = OccurrenceUsagePrefix
//                    ( 'connection' UsageDeclaration ValuePart? ( 'connect' ConnectorPart )?
//                    | 'connect' ConnectorPart ) UsageBody
// rejects: PARSE-EXPECTED on: connect;
part def P {
	connect;
}
