// REJECTED BY RULE. A message names its source and target events together: the source
// after `from`, then the target after `to` (7.16.2, receipt 13d6f883). Neither is written
// alone.
// SysML 8.2.2.16 — MessageDeclaration = UsageDeclaration ValuePart?
//                    ( 'of' FlowPayloadFeatureMember )?
//                    ( 'from' MessageEventMember 'to' MessageEventMember )?
//                  | MessageEventMember 'to' MessageEventMember
// rejects: PARSE-EXPECTED on: message setSpeedMessage from driver.sendSetSpeed;
part def Vehicle {
	message setSpeedMessage from driver.sendSetSpeed;
}
