// REJECTED BY RULE. A binding is not an occurrence usage, so it takes UsagePrefix and not
// OccurrenceUsagePrefix: `individual`, `snapshot` and `timeslice` do not apply to it.
// SysML 7.13.3      — "a binding is not a kind of occurrence usage (unlike a regular
//                     connection usage), so the notations for time slices, snapshots and
//                     individuals (described in 7.9) do not apply to it" (receipt 6db87b41)
// SysML 8.2.2.13.2  — BindingConnectorAsUsage = UsagePrefix ( 'binding' UsageDeclaration )? ...
// SysML 8.2.2.9.2   — OccurrenceUsagePrefix = ... BasicUsagePrefix 'individual'? PortionKind? ...
// rejects: PARSE-UNEXPECTED on: snapshot bind a = b;
part def P {
	snapshot bind a = b;
}
