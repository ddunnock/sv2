// REJECTION BY ABSENCE, not by rule. The pinned corpus writes '->' 21 times across
// 13 files. FunctionOperationExpression is part of the postfix layer and is not
// implemented; its body argument would also need BodyExpression.
// KerML 8.2.5.8.2 - FunctionOperationExpression =
//                     ownedRelationship += PrimaryArgumentMember
//                     '->' ownedRelationship += InstantiatedTypeMember
//                     ( BodyArgumentMember | FunctionReferenceArgumentMember
//                     | ArgumentList )
//                     ownedRelationship += EmptyResultMember
// Remove this case when FunctionOperationExpression lands.
attribute x = a->size();
