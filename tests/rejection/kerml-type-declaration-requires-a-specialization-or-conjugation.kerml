// REJECTED BY RULE. A TypeDeclaration writes one or more SpecializationParts or
// ConjugationParts. A classifier's may be omitted; a type's may not.
// KerML 8.2.4.1.1 — TypeDeclaration = ( isSufficient ?= 'all' )? Identification
//                       ( ownedRelationship += OwnedMultiplicity )?
//                       ( SpecializationPart | ConjugationPart )+
//                       TypeRelationshipPart*
// deviations.json records TypeDeclaration follow_spec: the clause's `+`, where the Pilot
// writes the group once and required either way.
// rejects: PARSE-EXPECTED on: type Singleton[1];
package Types {
	type Singleton[1];
}
