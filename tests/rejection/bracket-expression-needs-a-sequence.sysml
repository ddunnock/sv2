// REJECTED BY RULE. SequenceExpressionListMember is not optional, so the brackets may
// not be empty.
// KerML 8.2.5.8.2 — BracketExpression           = PrimaryArgumentMember '['
//                                                 SequenceExpressionListMember ']'
//                   SequenceExpressionListMember = ownedRelatedElement
//                                                    += SequenceExpressionList
//                   SequenceExpressionList       = OwnedExpression ','?
//                                                | SequenceOperatorExpression
// Neither alternative of SequenceExpressionList derives the empty string, so `[]` is
// not a bracket expression with nothing in it — it is not one at all.
attribute x = 1200 [];
