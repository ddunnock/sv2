// REJECTED BY RULE. The left operand of `meta` is a MetadataArgumentMember, and
// that reaches a QualifiedName and not an OwnedExpression — so a
// MetaclassificationExpression cannot itself be one, and the operator does not
// chain. `y meta A` parses; the classification operators at the same tier 8 DO
// chain, because their left operand is an ArgumentMember.
// KerML 8.2.5.8.1 — MetaclassificationExpression =
//                     ownedRelationship += MetadataArgumentMember ...
// KerML 8.2.5.8.1 — MetadataArgument = ownedRelationship += MetadataValue
// KerML 8.2.5.8.1 — MetadataValue = value = MetadataReference
// KerML 8.2.5.8.1 — MetadataReference = ownedRelationship += ElementReferenceMember
// KerML 8.2.5.8.3 — ElementReferenceMember = memberElement = [QualifiedName]
attribute x = y meta A meta B;
