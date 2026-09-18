// REJECTED BY ABSENCE. UsageExtensionKeyword is a PrefixMetadataMember, and prefix
// metadata is unimplemented everywhere in this parser.
// SysML 8.2.2.21.1 — SubjectUsage         = 'subject' UsageExtensionKeyword* Usage
// SysML 8.2.2.6.2  — UsageExtensionKeyword = ownedRelationship += PrefixMetadataMember
// The `*` makes zero of them the common case, so SubjectUsage is read without it and
// is left unmarked for coverage because of this gap. The sibling of
// tests/rejection/metadata-annotating-element-is-not-implemented.sysml. Removing this
// rejection means implementing PrefixMetadataMember, which removes several others.
requirement def R {
	subject #approved vehicle : Vehicle;
}
