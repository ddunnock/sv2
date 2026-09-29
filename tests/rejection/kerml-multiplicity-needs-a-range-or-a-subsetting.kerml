// REJECTED BY RULE. A multiplicity declaration writes either a range or a subsetting
// of another multiplicity after its Identification; with neither, it is no Multiplicity.
// KerML 8.2.5.11 — Multiplicity       = MultiplicitySubset | MultiplicityRange
//                  MultiplicitySubset = 'multiplicity' Identification Subsets TypeBody
//                  MultiplicityRange  = 'multiplicity' Identification MultiplicityBounds
//                                       TypeBody
// rejects: PARSE-EXPECTED on: multiplicity zeroOrMore;
package Multiplicities {
	multiplicity zeroOrMore;
}
