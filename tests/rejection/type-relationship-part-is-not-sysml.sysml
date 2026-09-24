// REJECTED BY RULE. TypeRelationshipPart is KerML's (KerML 8.2.4.1.1): no SysML
// definition or usage declaration reaches it, and a .sysml file is read against the
// SysML grammar alone (ADR-0014).
// SysML 8.2.2.6.1 — DefinitionDeclaration = Identification SubclassificationPart?
// rejects: PARSE-EXPECTED on: part def OurBicycle unions MyBike, YourBike;
part def OurBicycle unions MyBike, YourBike;
