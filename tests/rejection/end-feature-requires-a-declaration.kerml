// Rule: an end feature written without the `feature` keyword must declare something.
// KerML 8.2.4.3.1 — Feature's second alternative is
//   ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration
// with no `?` on FeatureDeclaration, and FeatureDeclaration cannot be empty: its
// FeatureIdentification needs a name, and its other alternatives need a part.
// The pilot Xtext writes Identification, whose parts are both optional, and so
// accepts this; the specification does not. Recorded on the FeatureDeclaration unit.
end;
