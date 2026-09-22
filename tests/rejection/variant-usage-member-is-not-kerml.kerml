// REJECTED BY RULE. KerML has no variants: VariantUsageMember is a SysML production, and
// no KerML body element reaches it (ADR-0014).
// KerML 8.2.4.1  — TypeBodyElement = NonFeatureMember | FeatureMember | AliasMember
//                                  | Import
// SysML 8.2.2.6.1 — VariantUsageMember = MemberPrefix 'variant' VariantUsageElement
// rejects: PARSE-UNEXPECTED on: variant x;
classifier C {
	variant x;
}
