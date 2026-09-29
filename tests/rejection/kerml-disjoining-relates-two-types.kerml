// REJECTED BY RULE. A Disjoining declaration relates one type to one other. A list of
// disjoining types is written in a type's own declaration, as its DisjoiningPart.
// KerML 8.2.4.1.4 — Disjoining = ( 'disjoining' Identification )?
//                       'disjoint' ( [QualifiedName] | FeatureChain )
//                       'from' ( [QualifiedName] | FeatureChain )
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: disjoint Mammal from Mineral, Vegetable;
package Kingdoms {
	disjoint Mammal from Mineral, Vegetable;
}
