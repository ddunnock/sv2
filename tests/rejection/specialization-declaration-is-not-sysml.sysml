// REJECTED BY RULE. Specialization declared on its own, `subtype A :> B;`, is a KerML
// NonFeatureElement. SysML's PackageBodyElement reaches no such production, and a
// .sysml file is parsed from SysML's start symbol alone (ADR-0014).
// KerML 8.2.3.4.3 — NonFeatureElement = ... | Specialization | ...
// KerML 8.2.4.1.2 — Specialization    = ( 'specialization' Identification )?
//                                        'subtype' SpecificType SPECIALIZES GeneralType
//                                        RelationshipBody
// rejects: PARSE-UNEXPECTED on: subtype C specializes A;
package Types {
	subtype C specializes A;
}
