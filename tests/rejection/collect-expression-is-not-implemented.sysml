// REJECTED BY ABSENCE, not by rule.
// KerML 8.2.5.8.2 — CollectExpression = ownedRelationship += PrimaryArgumentMember '.'
//                                       ownedRelationship += BodyArgumentMember
// The second production that writes a bare '.' after a primary, and the second reason
// at_feature_chain asks for a NAME after the dot rather than accepting any '.'. A
// BodyArgumentMember reaches BodyExpression, which opens on '{': a chain that took any
// '.' would read `things.` and leave the body to be misread after it.
attribute x = things.{ a > 1 };
