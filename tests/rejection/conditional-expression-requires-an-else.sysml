// REJECTED BY RULE. Both branches are in the production; the `else` is not
// optional, because a ConditionalExpression owns two ArgumentExpressionMembers.
// KerML 8.2.5.8.1 — ConditionalExpression = operator = 'if'
//                     ownedRelationship += ArgumentMember '?'
//                     ownedRelationship += ArgumentExpressionMember 'else'
//                     ownedRelationship += ArgumentExpressionMember
//                     ownedRelationship += EmptyResultMember
attribute x = if a? b;
