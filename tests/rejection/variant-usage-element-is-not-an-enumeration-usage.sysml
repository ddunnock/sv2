// REJECTED BY RULE. VariantUsageElement is UsageElement less DefaultReferenceUsage,
// EnumerationUsage and ExtendedUsage. The rule is the grammar's; 8.4.2.3 (receipt
// 4ad35baf) excludes enumerations from being variations, which is consistent with it.
// SysML 8.2.2.6.4 — VariantUsageElement = VariantReference | ReferenceUsage
//                   | AttributeUsage | … — with no EnumerationUsage alternative.
// rejects: PARSE-EXPECTED on: variant enum e;
variation attribute def A {
	variant enum e;
}
