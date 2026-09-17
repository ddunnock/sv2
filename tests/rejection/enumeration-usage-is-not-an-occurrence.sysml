// Rule: an enumeration's prefix is a UsagePrefix, which carries no PortionKind and
// no 'individual'. Only an OccurrenceUsagePrefix does, and an enumeration is not an
// occurrence — the same distinction AttributeUsage draws.
// SysML 8.2.2.8   — EnumerationUsage = UsagePrefix 'enum' Usage
// SysML 8.2.2.6.2 — UsagePrefix = UnextendedUsagePrefix UsageExtensionKeyword*
// SysML 8.2.2.9.2 — OccurrenceUsagePrefix = ( EndUsagePrefix
//                     | BasicUsagePrefix 'individual'? PortionKind? ) ...
snapshot enum green;
