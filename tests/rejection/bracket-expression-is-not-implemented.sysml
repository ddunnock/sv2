// REJECTION BY ABSENCE, not by rule. `1200 [kg]` is the quantity form and the
// pinned corpus writes a bracketed unit 297 times. BracketExpression is part of
// the postfix layer and is not implemented.
// NOTE the '[' here is NOT the '[' of a MultiplicityPart, which IS implemented:
// `part p[2];` parses. A '[' after a declared feature name is a multiplicity and a
// '[' after an expression operand is a BracketExpression, and telling the two
// apart is the work this case is waiting on.
// KerML 8.2.5.8.2 - BracketExpression =
//                     ownedRelationship += PrimaryArgumentMember
//                     '[' ownedRelationship += SequenceExpressionListMember ']'
// Remove this case when BracketExpression lands.
attribute x = 1200 [kg];
