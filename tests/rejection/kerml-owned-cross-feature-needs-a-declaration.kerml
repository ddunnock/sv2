// REJECTED BY RULE. What stands between an end feature's `end` and its keyword is an
// owned cross feature, a BasicFeaturePrefix AND a FeatureDeclaration: a prefix alone
// declares nothing. EndFeaturePrefix writes no `derived`, so it cannot absorb it.
// KerML 8.2.4.3.1 — EndFeaturePrefix  = ( isConstant ?= 'const' )? isEnd ?= 'end'
//                   OwnedCrossFeature = BasicFeaturePrefix FeatureDeclaration
// rejects: PARSE-EXPECTED on: end derived feature cart : ShoppingCart[1];
assoc Selection {
	end derived feature cart : ShoppingCart[1];
}
