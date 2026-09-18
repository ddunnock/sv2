// REJECTED BY RULE. The ']' is not optional.
// KerML 8.2.5.8.2 — BracketExpression = PrimaryArgumentMember '['
//                                       SequenceExpressionListMember ']'
// The sibling of tests/rejection/multiplicity-range-unclosed.sysml, one role along:
// that file holds the declaration-side '[' and this one holds the expression-side '['.
attribute x = 1200 [kg;
