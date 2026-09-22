// REJECTED BY RULE. ReferenceUsage is the one usage whose prefix takes no user-defined
// keyword: it writes RefPrefix, not UsagePrefix. `ref #X y;` is an ExtendedUsage, whose
// `ref` is BasicUsagePrefix's; `#X ref y;` is neither.
// SysML 8.2.2.6.2 — ReferenceUsage = ( EndUsagePrefix | RefPrefix ) 'ref' Usage
// SysML 8.2.2.27  — ExtendedUsage = UnextendedUsagePrefix UsageExtensionKeyword+ Usage
// rejects: PARSE-UNEXPECTED on: #Approved ref driver;
part def Vehicle {
	#Approved ref driver;
}
