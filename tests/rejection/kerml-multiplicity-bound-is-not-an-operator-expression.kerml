// REJECTED BY RULE. A multiplicity bound is a literal or a feature reference, not an
// operator expression: MultiplicityExpressionMember names exactly two alternatives.
// KerML 8.2.5.11 — MultiplicityBounds = '[' ( MultiplicityExpressionMember '..' )?
//                                           MultiplicityExpressionMember ']'
//                  MultiplicityExpressionMember =
//                      ownedRelatedElement += ( LiteralExpression | FeatureReferenceExpression )
// rejects: PARSE-EXPECTED on: classifier MyBike [1 + 1];
package P {
	classifier MyBike [1 + 1];
}
