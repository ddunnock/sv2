// REJECTED BY RULE. An owned cross feature stands between `end` and a KIND keyword:
// "Such owned cross features are declared between the end and kind keywords of the
// association end declarations (and, in this case, the kind keyword is required)"
// (7.13.2, receipt 5a3a8867). Without the kind keyword the line is a DefaultReferenceUsage,
// whose bare `end` (deviation DefaultReferenceUsage) owns no cross feature, and whose
// declaration `[1]` cannot be followed by a name.
// SysML 8.2.2.6.2 — EndUsagePrefix = 'end' OwnedCrossFeatureMember?
// SysML 8.2.2.6.3 — DefaultReferenceUsage = RefPrefix Usage
// rejects: PARSE-UNEXPECTED on: end [1] hub ::> x;
connection def D {
	end [1] hub ::> x;
}
