// REJECTED BY RULE. A multiplicity bound is a literal or a name and NOT an
// expression: the clause names two alternatives, neither of them an operator
// expression, so the `+` has nowhere to go and the `]` is reported missing.
// SysML 8.2.2.6.6 — MultiplicityExpressionMember =
//                     ownedRelatedElement += ( LiteralExpression
//                                            | FeatureReferenceExpression )
// `part p[1];` and `part p[n];` both parse, so this is the rule and not an absence.
part p[1+1];
