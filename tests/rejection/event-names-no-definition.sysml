// REJECTED BY RULE. `event` is a kind keyword of usages only: SysML declares event
// occurrences by usage (7.9.5) and states no `event def` production.
// SysML 8.2.2.9.2 — EventOccurrenceUsage = OccurrenceUsagePrefix 'event' ...
//        8.2.2.9.1 — OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' ...
// rejects: PARSE-EXPECTED on: event def Request;
part def Client {
	event def Request;
}
