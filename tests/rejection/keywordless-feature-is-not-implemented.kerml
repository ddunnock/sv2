// REJECTED BY ABSENCE, not by rule. Feature has two ways in, and this parser reads
// one. The second writes no keyword at all and lets the declaration carry it, which
// is why a BasicFeaturePrefix may be empty and the FeatureDeclaration after it may
// not be.
// KerML 8.2.4.3.1 — Feature =
//     ( FeaturePrefix ( 'feature' | PrefixMetadataMember ) FeatureDeclaration?
//     | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration
//     ) ValuePart? TypeBody
// `feature vitesse : Speed;` parses, which is what makes this an absence and not a
// rule. Remove this case when the second alternative lands.
vitesse : Speed;
