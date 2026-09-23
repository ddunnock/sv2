// REJECTED BY RULE. A classifier's multiplicity follows its Identification; it does not
// precede it.
// KerML 8.2.4.2.1 — ClassifierDeclaration = ( isSufficient ?= 'all' )? Identification
//                                           ( ownedRelationship += OwnedMultiplicity )?
//                                           ( SuperclassingPart | ConjugationPart )?
//                                           TypeRelationshipPart*
// rejects: PARSE-EXPECTED on: classifier [1] MyBike specializes Bicycle;
package P {
	classifier [1] MyBike specializes Bicycle;
}
