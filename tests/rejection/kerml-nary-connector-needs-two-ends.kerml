// REJECTED BY RULE. The parenthesised list of connector ends names at least two: the
// production writes two ConnectorEndMembers before its optional repetition.
// KerML 8.2.5.5.1 — NaryConnectorDeclaration = FeatureDeclaration?
//                   '(' ConnectorEndMember ',' ConnectorEndMember
//                       ( ',' ConnectorEndMember )* ')'
// rejects: PARSE-EXPECTED on: connector mount[2] : Mounting (axle);
struct WheelAssembly {
	connector mount[2] : Mounting (axle);
}
