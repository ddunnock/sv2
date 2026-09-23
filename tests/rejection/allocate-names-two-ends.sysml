// REJECTED BY RULE. `allocate` is followed by a ConnectorPart, which names two ends joined
// by `to`, or two or more in parentheses; one end alone is neither.
// SysML 8.2.2.15   — AllocationUsageDeclaration = ... | 'allocate' ConnectorPart
// SysML 8.2.2.13.1 — ConnectorPart = BinaryConnectorPart | NaryConnectorPart
//                    BinaryConnectorPart = ConnectorEndMember 'to' ConnectorEndMember
// rejects: PARSE-EXPECTED on: allocate torqueGenerator;
part def Vehicle {
	allocate torqueGenerator;
}
