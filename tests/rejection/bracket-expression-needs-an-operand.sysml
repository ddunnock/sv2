// REJECTED BY RULE, and this is the case that keeps the two roles of '[' apart.
// KerML 8.2.5.8.2 — BracketExpression = ownedRelationship += PrimaryArgumentMember
//                                       operator = '['
//                                       ownedRelationship += SequenceExpressionListMember
//                                       ']'
// SysML 8.2.2.6.6 — MultiplicityRange = '[' ( MultiplicityExpressionMember '..' )?
//                                       MultiplicityExpressionMember ']'
// The two productions write the same two tokens around the same kind of content, and
// exactly one thing separates them: a BracketExpression has an OPERAND before the '['
// and a MultiplicityRange does not. `= [kg]` has no operand, so it is not a bracket
// expression — and it is in a value position, where no multiplicity may appear, so it
// is not a multiplicity either. Accepting it would mean the parser had stopped telling
// the roles apart by position, which is the whole resolution recorded for this
// question. `attribute mass : MassValue[1] = 1200 [kg];` writes BOTH roles on one line
// and parses; that case lives in crates/sv2-syntax/tests/parser.rs.
attribute x = [kg];
