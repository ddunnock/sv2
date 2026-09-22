// REJECTED BY RULE. An interface end may open with a cross multiplicity, but writes none
// after the port it references. Deviation ConnectorEnd-trailing-multiplicity admits that
// form for ConnectorEnd alone, on 7.13.2's example; InterfaceEnd is another production.
// SysML 8.2.2.14.2 — InterfaceEnd = ( ownedRelationship += OwnedCrossMultiplicityMember )?
//                      ( declaredName = NAME REFERENCES )?
//                      ownedRelationship += OwnedReferenceSubsetting
// rejects: PARSE-EXPECTED on: interface hub : WheelHub connect wheel.hubPort[1] to axle.hubPort;
part vehicle {
	interface hub : WheelHub connect wheel.hubPort[1] to axle.hubPort;
}
