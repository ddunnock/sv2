// REJECTION BY ABSENCE, not by rule. `a.b` is well-formed SysML and the corpus
// writes it constantly. FeatureChainExpression is the postfix '.' of
// PrimaryExpression and is not implemented: the operator core reads the fifteen
// precedence tiers of KerML 8.2.5.8.1 and stops before the postfix layer of
// 8.2.5.8.2.
// KerML 8.2.5.8.2 - PrimaryExpression = FeatureChainExpression
//                                     | NonFeatureChainPrimaryExpression
// KerML 8.2.5.8.2 - FeatureChainExpression =
//                     ownedRelationship += NonFeatureChainPrimaryArgumentMember
//                     '.' ownedRelationship += FeatureChainMember
// Remove this case when FeatureChainExpression lands.
attribute x = a.b;
