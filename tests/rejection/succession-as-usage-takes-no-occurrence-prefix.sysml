// REJECTED BY RULE. A succession is not an occurrence usage, so it takes UsagePrefix and
// not OccurrenceUsagePrefix: `individual`, `snapshot` and `timeslice` do not apply to it.
// SysML 7.13.5      — "a succession is not a kind of occurrence usage (unlike a regular
//                     connection usage), so the notations for time slices, snapshots and
//                     individuals (described in 7.9) do not apply to it" (receipt 2abd302c)
// SysML 8.2.2.13.3  — SuccessionAsUsage = UsagePrefix ( 'succession' UsageDeclaration )? ...
// SysML 8.2.2.9.2   — OccurrenceUsagePrefix = ... BasicUsagePrefix 'individual'? PortionKind? ...
// rejects: PARSE-UNEXPECTED on: snapshot first a then b;
part def P {
	snapshot first a then b;
}
