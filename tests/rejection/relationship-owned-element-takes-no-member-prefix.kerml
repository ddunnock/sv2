// REJECTED BY RULE. An element in a KerML relationship body is the relationship's
// ownedRelatedElement, owned with no Membership between them, so no MemberPrefix — and no
// visibility — is written before it.
// KerML 8.2.3.1 — RelationshipOwnedElement = ownedRelatedElement += OwnedRelatedElement
//                                          | ownedRelationship += OwnedAnnotation
//                 OwnedRelatedElement = NonFeatureElement | FeatureElement
// rejects: PARSE-UNEXPECTED on: private feature e;
dependency a to b {
	private feature e;
}
