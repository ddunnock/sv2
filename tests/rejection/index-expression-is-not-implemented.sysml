// REJECTION BY ABSENCE, not by rule. The pinned corpus writes '#(' 45 times, among
// them "Named Collection Members Example/VehicleTanks.kerml" line 19.
// IndexExpression is part of the postfix layer and is not implemented.
// KerML 8.2.5.8.2 - IndexExpression =
//                     ownedRelationship += PrimaryArgumentMember
//                     '#' '(' ownedRelationship += SequenceExpressionListMember ')'
// Remove this case when IndexExpression lands.
attribute x = tanks#(1);
