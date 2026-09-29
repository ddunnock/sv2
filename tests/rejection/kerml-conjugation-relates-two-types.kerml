// REJECTED BY RULE. A Conjugation declaration relates one conjugated type to one
// original type; the production writes no list on either side.
// KerML 8.2.4.1.3 — Conjugation = ( 'conjugation' Identification )?
//                       'conjugate' ( [QualifiedName] | FeatureChain )
//                       CONJUGATES ( [QualifiedName] | FeatureChain )
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: conjugate Conjugate1 conjugates Original, Other;
package Types {
	conjugate Conjugate1 conjugates Original, Other;
}
