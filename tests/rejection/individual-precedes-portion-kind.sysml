// REJECTED BY RULE. `individual` is written before a portion kind, never after it: "the
// keyword individual, placed after any of the other usage property keywords ... but
// before a timeslice or snapshot keyword (if any)" (7.9.4, receipt 8c84370d).
// SysML 8.2.2.9.2 — PortionUsage = BasicUsagePrefix ( isIndividual ?= 'individual' )?
//                     portionKind = PortionKind UsageExtensionKeyword* Usage
// rejects: PARSE-UNEXPECTED on: snapshot individual s;
occurrence def Flight {
	snapshot individual s;
}
