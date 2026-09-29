// REJECTED BY RULE. A metadata access suffixes `.metadata` to the qualified name of an
// element, never to a feature chain: its ElementReferenceMember is a QualifiedName alone,
// and no postfix production of PrimaryExpression takes `.metadata` after one (the
// Pilot's postfix loop, KerMLExpressions.xtext:299-322, offers none either).
// KerML 8.2.5.8.3 — MetadataAccessExpression =
//                       ownedRelationship += ElementReferenceMember '.' 'metadata'
//                   ElementReferenceMember = memberElement = [QualifiedName]
// rejects: PARSE-EXPECTED on: attribute m = vehicle.engine.metadata;
part def Vehicle {
	attribute m = vehicle.engine.metadata;
}
