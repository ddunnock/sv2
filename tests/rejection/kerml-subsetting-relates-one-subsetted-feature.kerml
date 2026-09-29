// REJECTED BY RULE. A Subsetting declaration relates one subsetting feature to one
// subsetted feature. A list of subsetted features is written in a feature's own
// declaration, as its owned subsettings, not here.
// KerML 8.2.4.3.3 — Subsetting = ( 'specialization' Identification )?
//                       'subset' SpecificType SUBSETS GeneralType RelationshipBody
// rejects: PARSE-EXPECTED on: subset rearWheels subsets wheels, driveWheels;
package Features {
	subset rearWheels subsets wheels, driveWheels;
}
