// REJECTED BY RULE. An allocation usage's declaration takes no ValuePart: unlike
// ConnectionUsage's first alternative, AllocationUsageDeclaration writes UsageDeclaration
// straight to the optional `allocate`.
// SysML 8.2.2.15 — AllocationUsageDeclaration = 'allocation' UsageDeclaration
//                    ( 'allocate' ConnectorPart )? | 'allocate' ConnectorPart
// SysML 8.2.2.13.1 — ConnectionUsage = ... 'connection' UsageDeclaration ValuePart? ...
// rejects: PARSE-EXPECTED on: allocation torqueGenAlloc : LogicalToPhysical = defaultAlloc;
part def Vehicle {
	allocation torqueGenAlloc : LogicalToPhysical = defaultAlloc;
}
