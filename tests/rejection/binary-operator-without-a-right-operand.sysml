// REJECTED BY RULE. A BinaryOperatorExpression owns two ArgumentMembers, so an
// operator with nothing after it is not one.
// KerML 8.2.5.8.1 — BinaryOperatorExpression =
//                     ownedRelationship += ArgumentMember
//                     operator = BinaryOperator
//                     ownedRelationship += ArgumentMember
//                     ownedRelationship += EmptyResultMember
attribute x = 1 + ;
