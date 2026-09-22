// REJECTED BY RULE. A KerML relationship body owns elements and annotations, not
// memberships: AliasMember is a NamespaceBodyElement (8.2.3.4.1) and no
// OwnedRelatedElement. Import is excluded by the same rule, and tests/kerml.rs holds it.
// KerML 8.2.3.1 — RelationshipBody = ';' | '{' RelationshipOwnedElement* '}'
//                 OwnedRelatedElement = NonFeatureElement | FeatureElement
// rejects: PARSE-UNEXPECTED on: alias X for Y;
dependency a to b {
	alias X for Y;
}
