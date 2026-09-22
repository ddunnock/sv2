// REJECTED BY RULE. KerML's TypedBy takes an OwnedFeatureTyping alone; the `~` typing is
// SysML's ConjugatedPortTyping, and KerML has no ports (ADR-0014).
// KerML 8.2.4.3.1 — TypedBy = TYPED_BY ownedRelationship += OwnedFeatureTyping
// SysML 8.2.2.12  — ConjugatedPortTyping = '~' originalPortDefinition = ~[QualifiedName]
// rejects: PARSE-EXPECTED on: feature f : ~T;
feature f : ~T;
