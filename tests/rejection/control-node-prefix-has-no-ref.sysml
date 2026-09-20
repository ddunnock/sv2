// REJECTED BY RULE. ControlNodePrefix opens on RefPrefix, not BasicUsagePrefix, so a
// control node takes `abstract`, `derived`, a direction and the rest -- but not `ref`,
// which only BasicUsagePrefix adds.
// SysML 8.2.2.17.3  — ControlNodePrefix = RefPrefix 'individual'? PortionKind?
//                                         UsageExtensionKeyword*
// SysML 8.2.2.6.2   — BasicUsagePrefix = RefPrefix 'ref'?
// rejects: PARSE-EXPECTED on: ref merge m;
action def A {
	ref merge m;
}
