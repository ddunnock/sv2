// REJECTED BY RULE. A Redefinition declaration is written with REDEFINES, `:>>` or
// `redefines`. `:>` is SUBSETS's symbol, and SPECIALIZES's, never REDEFINES's.
// KerML 8.2.4.3.4 — Redefinition = ( 'specialization' Identification )?
//                       'redefinition' SpecificType REDEFINES GeneralType
//                       RelationshipBody
// KerML 8.2.2.7   — REDEFINES = ':>>' | 'redefines'
// rejects: PARSE-EXPECTED on: redefinition Vehicle::vin :> legalIdentification;
package Features {
	redefinition Vehicle::vin :> legalIdentification;
}
