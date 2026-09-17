// Rule: an attribute's prefix is a UsagePrefix, which carries no PortionKind. Only
// an OccurrenceUsagePrefix does, and an attribute is not an occurrence.
// SysML 8.2.2.7   — AttributeUsage = UsagePrefix 'attribute' Usage
// SysML 8.2.2.6.2 — UsagePrefix = UnextendedUsagePrefix UsageExtensionKeyword*
// SysML 8.2.2.9.2 — OccurrenceUsagePrefix = ( EndUsagePrefix
//                     | BasicUsagePrefix 'individual'? PortionKind? ) ...
snapshot attribute mass : MassValue;
