// REJECTED BY RULE. The right operand of a classification test is a TypeReference
// over a QualifiedName, so a literal cannot be one. `y istype T` parses.
// KerML 8.2.5.8.1 — ClassificationExpression = ( ownedRelationship += ArgumentMember )?
//                     ( operator = ClassificationTestOperator
//                       ownedRelationship += TypeReferenceMember | ... )
// KerML 8.2.5.8.1 — TypeReference = ownedRelationship += ReferenceTyping
// KerML 8.2.5.8.1 — ReferenceTyping = type = [QualifiedName]
attribute x = y istype 1;
