// REJECTED BY RULE. A requirement body reaches DefinitionBodyItem, not ActionBodyItem, so
// it does not admit `first` — however much a requirement reads like a behaviour.
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember
//                                        | RequirementConstraintMember | …
// SysML 8.2.2.6.1  — DefinitionBodyItem has no ActionBodyItem alternative.
// This is the direction a comment in parser.rs once had wrong: it said ActionBodyItem was
// reached by RequirementBodyItem and DefinitionBodyItem. Neither reaches it. What does is
// ActionBody, CalculationBodyItem (8.2.2.19), CaseBodyItem (8.2.2.22),
// ActionBodyParameter (8.2.2.17.7) and the four Transition*ActionUsage bodies (8.2.2.18.3).
requirement def R {
	first start;
}
