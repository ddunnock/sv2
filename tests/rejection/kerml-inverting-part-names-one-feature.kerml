// REJECTED BY RULE. An InvertingPart names one inverting feature, not a list.
// KerML 8.2.4.3.1 — InvertingPart         = 'inverse' 'of' OwnedFeatureInverting
// KerML 8.2.4.3.6 — OwnedFeatureInverting = invertingFeature = [QualifiedName]
//                                         | invertingFeature = OwnedFeatureChain
// A second inverse is a second part, `inverse of g inverse of h`; a `,` continues
// nothing a FeatureDeclaration ends in.
// rejects: PARSE-EXPECTED on: feature parents : Person[*] inverse of children, offspring;
classifier Person {
	feature children : Person[*];
	feature offspring : Person[*];
	feature parents : Person[*] inverse of children, offspring;
}
