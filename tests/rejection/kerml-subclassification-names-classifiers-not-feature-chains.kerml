// REJECTED BY RULE. A Subclassification relates two classifiers by name. Unlike a
// Specialization's SpecificType and GeneralType, neither side admits a feature chain.
// KerML 8.2.4.2.2 — Subclassification = ( 'specialization' Identification )?
//                       'subclassifier' subclassifier = [QualifiedName]
//                       SPECIALIZES superclassifier = [QualifiedName]
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: subclassifier C specializes b.f;
package Classifiers {
	subclassifier C specializes b.f;
}
