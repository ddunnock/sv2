// REJECTED BY RULE. The parenthesised form names at least two ends: "a comma-separated
// list, between parentheses" (7.13.2, receipt 5a3a8867), and the production writes the
// first comma outside its repetition.
// SysML 8.2.2.13.1 — NaryConnectorPart = '(' ConnectorEndMember ',' ConnectorEndMember
//                                        ( ',' ConnectorEndMember )* ')'
// rejects: PARSE-EXPECTED on: connect (a);
part def P {
	connect (a);
}
