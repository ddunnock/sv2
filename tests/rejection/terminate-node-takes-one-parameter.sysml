// REJECTED BY RULE. A TerminateAction "has a single parameter that identifies a
// terminated occurrence" (7.17.10, receipt aa4c3e77), and the production writes at most
// one NodeParameterMember before its ActionBody.
// SysML 8.2.2.17.6 — TerminateNode = OccurrenceUsagePrefix ActionNodeUsageDeclaration?
//                                    'terminate' NodeParameterMember? ActionBody
// rejects: PARSE-EXPECTED on: terminate process processor;
action def TerminateProcessing {
	terminate process processor;
}
