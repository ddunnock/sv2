// REJECTION BY ABSENCE, not by rule. InvocationExpression is a BaseExpression
// alternative and needs ArgumentList, which is not implemented.
// KerML 8.2.5.8.3 - InvocationExpression =
//                     ownedRelationship += InstantiatedTypeMember ArgumentList
//                     ownedRelationship += EmptyResultMember
// KerML 8.2.5.8.3 - ArgumentList =
//                     '(' ( PositionalArgumentList | NamedArgumentList )? ')'
// Remove this case when InvocationExpression lands.
attribute x = f(1, 2);
