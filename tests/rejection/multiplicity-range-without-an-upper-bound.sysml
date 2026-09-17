// REJECTED BY RULE. The UPPER bound is the required one and the lower is optional,
// so `[1..]` is missing the bound the clause does not make optional. `[1]` is that
// same required bound written alone.
// SysML 8.2.2.6.6 — MultiplicityRange =
//                     '( ownedRelationship += MultiplicityExpressionMember '..' )?
//                       ownedRelationship += MultiplicityExpressionMember ']'
part p[1..];
