// REJECTED BY RULE. `snapshot` and `timeslice` declare portions of an occurrence, which are
// usages; no production writes a portion kind before `def`.
// SysML 8.2.2.9.2 — PortionUsage = BasicUsagePrefix 'individual'? PortionKind
//                     UsageExtensionKeyword* Usage
// SysML 8.2.2.9.1 — IndividualDefinition = BasicDefinitionPrefix? 'individual'
//                     DefinitionExtensionKeyword* 'def' Definition EmptyMultiplicityMember
// rejects: PARSE-UNEXPECTED on: snapshot def Takeoff;
snapshot def Takeoff;
