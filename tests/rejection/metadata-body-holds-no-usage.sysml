// REJECTED BY RULE. A metadata usage's body redefines the features of its definition;
// it does not declare new usages. MetadataBody admits a DefinitionMember, an AliasMember,
// an Import and a MetadataBodyUsageMember, whose MetadataBodyUsage opens on the name of
// the feature it redefines -- and `attribute` is a keyword, not a name (8.2.2.1.2).
// "These nested reference usages of a metadata usage must always have the same names as
// the names of the features of its metadata definition" (7.27.2, receipt 66e5d6b3).
// SysML 8.2.2.27 — MetadataBody = ';' | '{' ( DefinitionMember | MetadataBodyUsageMember
//                                          | AliasMember | Import )* '}'
// rejects: PARSE-UNEXPECTED on: attribute approved : Boolean = true;
part def Design {
	@ApprovalAnnotation {
		attribute approved : Boolean = true;
	}
}
