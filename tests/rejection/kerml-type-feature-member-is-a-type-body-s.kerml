// REJECTED BY RULE. `member` introduces a TypeFeatureMember, one of a TypeBody's two
// feature members. A package body is a namespace body, whose members are a
// NonFeatureMember or a NamespaceFeatureMember, neither of which writes `member`.
// KerML 8.2.4.1.6 — TypeFeatureMember = MemberPrefix 'member' FeatureElement
// KerML 8.2.3.4.1 — NamespaceMember   = NonFeatureMember | NamespaceFeatureMember
// rejects: PARSE-UNEXPECTED on: member feature x featured by CC1_snapshots;
package TimeVaryingFeatures {
	member feature x featured by CC1_snapshots;
}
