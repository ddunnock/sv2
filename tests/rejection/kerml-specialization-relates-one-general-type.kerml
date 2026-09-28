// REJECTED BY RULE. A Specialization declaration relates one specific type to one
// general type. A list of general types is written in a type's own declaration, as
// its owned specializations, not here.
// KerML 8.2.4.1.2 — Specialization = ( 'specialization' Identification )?
//                       'subtype' SpecificType SPECIALIZES GeneralType
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: subtype C specializes A, B;
package Types {
	subtype C specializes A, B;
}
