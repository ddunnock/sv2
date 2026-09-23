// REJECTED BY RULE. A binary connector references its source after `from` and its target
// after `to` (7.4.6.2, receipt d8abbbc3): both ends, the `to` between them.
// KerML 8.2.5.5.1 — BinaryConnectorDeclaration = ( FeatureDeclaration? 'from'
//                                                | isSufficient ?= 'all' 'from'? )?
//                                                ConnectorEndMember 'to' ConnectorEndMember
// rejects: PARSE-EXPECTED on: connector mount : Mounting from axle;
struct WheelAssembly {
	connector mount : Mounting from axle;
}
