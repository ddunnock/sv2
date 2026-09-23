// REJECTED BY RULE. A classifier declaration owns at most one multiplicity.
// KerML 8.2.4.2.1 — ClassifierDeclaration = … Identification
//                                           ( ownedRelationship += OwnedMultiplicity )? …
// rejects: PARSE-EXPECTED on: classifier MyBike [1] [0..1];
package P {
	classifier MyBike [1] [0..1];
}
