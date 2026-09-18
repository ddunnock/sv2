// REJECTED BY RULE. UsageElement is not optional — the `+=` is on a mandatory term, so
// `return` alone is the keyword with nothing to own.
// SysML 8.2.2.19  — ReturnParameterMember = MemberPrefix? 'return'
//                                           ownedRelatedElement += UsageElement
// SysML 8.2.2.5.2 — UsageElement = NonOccurrenceUsageElement | OccurrenceUsageElement
// A `return` with no usage is also what a parser that read the keyword and then took
// whatever followed would accept, which is why this is by rule and not by absence.
calc def C {
	return;
}
