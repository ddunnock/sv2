// REJECTED BY RULE. A message's payload comes before its events: "If they are included,
// then they follow the payload specification (if any)" (7.16.2, receipt 13d6f883).
// SysML 8.2.2.16 — MessageDeclaration = UsageDeclaration ValuePart?
//                    ( 'of' FlowPayloadFeatureMember )?
//                    ( 'from' MessageEventMember 'to' MessageEventMember )?
//                  | MessageEventMember 'to' MessageEventMember
// rejects: PARSE-EXPECTED on: message setSpeedMessage from driver.sendSetSpeed to vehicle.receiveSetSpeed of SetSpeed;
part def Vehicle {
	message setSpeedMessage from driver.sendSetSpeed to vehicle.receiveSetSpeed of SetSpeed;
}
