// REJECTED BY RULE. An include use case usage is declared "using the kind keyword include
// use case instead of just use case", or "using just the keyword include" followed by a
// qualified name or feature chain (7.25.3, receipt 6f1b9dfd). `include use` is neither:
// `use` is reserved, so it is no reference, and without `case` it is no kind keyword.
// SysML 8.2.2.25 — IncludeUseCaseUsage = OccurrenceUsagePrefix 'include'
//                  ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                  | 'use' 'case' UsageDeclaration ) ValuePart? CaseBody
// rejects: PARSE-EXPECTED on: include use 'enter vehicle' { }
use case def 'Provide Transportation' {
	include use 'enter vehicle' { }
}
