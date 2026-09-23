// REJECTED BY RULE. A function operation names its function and then gives it exactly
// one argument form: a body, a function reference, or an argument list. None of the
// three is optional, so `a->size` with nothing after it is not an expression; the
// corpus writes `powerProfile->size()` (validation/10-Analysis and Trades/10d-Dynamics
// Analysis.sysml:65).
// KerML 8.2.5.8.2 - FunctionOperationExpression =
//                     ownedRelationship += PrimaryArgumentMember '->'
//                     ownedRelationship += InstantiatedTypeMember
//                     ( ownedRelationship += BodyArgumentMember
//                     | ownedRelationship += FunctionReferenceArgumentMember
//                     | ArgumentList )
//                     ownedRelationship += EmptyResultMember
// rejects: PARSE-EXPECTED on: attribute x = a->size;
attribute x = a->size;
