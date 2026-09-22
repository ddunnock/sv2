// REJECTED BY RULE. EndUsagePrefix and `BasicUsagePrefix 'individual'? PortionKind?` are
// ALTERNATIVES of OccurrenceUsagePrefix, so an end usage is not also individual, and its
// `individual` cannot begin an owned cross feature either: OwnedCrossFeature is
// BasicUsagePrefix UsageDeclaration, and `individual` is in neither.
// SysML 8.2.2.9.2 — OccurrenceUsagePrefix = ( EndUsagePrefix
//                   | BasicUsagePrefix 'individual'? PortionKind? ) UsageExtensionKeyword*
//                   (as deviation OccurrenceUsagePrefix, follow_xtext, reads it)
// SysML 8.2.2.6.2 — EndUsagePrefix = 'end' OwnedCrossFeatureMember?
// rejects: PARSE-EXPECTED on: end individual part p;
connection def D {
	end individual part p;
}
