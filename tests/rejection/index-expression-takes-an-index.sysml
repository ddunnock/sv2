// REJECTED BY RULE. An index expression's parentheses hold a SequenceExpressionList,
// which opens on an OwnedExpression, so `tanks#()` indexes by nothing. The corpus always
// writes an index: `power#(i)` (training/33. Analysis/Analysis Case Definition
// Example.sysml:61).
// KerML 8.2.5.8.2 - IndexExpression =
//                     ownedRelationship += PrimaryArgumentMember '#'
//                     '(' ownedRelationship += SequenceExpressionListMember ')'
//                   SequenceExpressionList = OwnedExpression ','? | SequenceOperatorExpression
// rejects: PARSE-EXPECTED on: attribute x = tanks#();
attribute x = tanks#();
