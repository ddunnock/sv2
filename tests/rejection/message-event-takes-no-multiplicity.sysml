// REJECTED BY RULE. A message event is a reference to an event occurrence and nothing
// more: it takes no multiplicity, unlike an `event` usage's FeatureSpecializationPart.
// SysML 8.2.2.16 — MessageEventMember = ownedRelatedElement += MessageEvent
//                  MessageEvent : EventOccurrenceUsage =
//                    ownedRelationship += OwnedReferenceSubsetting
// rejects: PARSE-EXPECTED on: message setSpeedMessage from driver.sendSetSpeed[1] to vehicle.receiveSetSpeed;
part def Vehicle {
	message setSpeedMessage from driver.sendSetSpeed[1] to vehicle.receiveSetSpeed;
}
