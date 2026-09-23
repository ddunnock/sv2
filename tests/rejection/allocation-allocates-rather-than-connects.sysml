// REJECTED BY RULE. An allocation usage names its ends after `allocate`, not `connect`:
// "Shorthand notations similar to those for connection usages ... may also be used for
// allocation usages, but using the keyword allocate instead of connect" (7.15.2,
// receipt 546a588c).
// SysML 8.2.2.15 — AllocationUsageDeclaration = 'allocation' UsageDeclaration
//                    ( 'allocate' ConnectorPart )? | 'allocate' ConnectorPart
// rejects: PARSE-EXPECTED on: allocation torqueGenAlloc : LogicalToPhysical connect torqueGenerator to powerTrain;
part def Vehicle {
	allocation torqueGenAlloc : LogicalToPhysical connect torqueGenerator to powerTrain;
}
