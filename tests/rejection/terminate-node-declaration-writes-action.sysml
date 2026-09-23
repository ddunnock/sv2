// REJECTED BY RULE. A terminate action is declared by `action` and a UsageDeclaration
// before the keyword terminate, and "if the declaration part is empty, then the action
// keyword may be omitted" (7.17.10, receipt aa4c3e77) -- the whole part, never the keyword
// alone. `t terminate;` names a declaration without writing `action`.
// SysML 8.2.2.17.6 — TerminateNode = OccurrenceUsagePrefix ActionNodeUsageDeclaration?
//                                    'terminate' NodeParameterMember? ActionBody
// SysML 8.2.2.17.2 — ActionNodeUsageDeclaration = 'action' UsageDeclaration?
// rejects: PARSE-EXPECTED on: stop terminate;
action def MonitoredActivity {
	stop terminate;
}
