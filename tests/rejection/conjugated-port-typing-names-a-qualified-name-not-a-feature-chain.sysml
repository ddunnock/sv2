// REJECTED BY RULE. A conjugated port typing names a port definition by a QualifiedName,
// resolved to the conjugated port definition it implicitly declares; unlike an
// OwnedFeatureTyping it has no OwnedFeatureChain alternative, so `~a.b` is no typing.
// SysML 8.2.2.12  — ConjugatedPortTyping = '~' originalPortDefinition = ~[QualifiedName]
// SysML 8.2.2.6.5 — OwnedFeatureTyping = [QualifiedName] | OwnedFeatureChain
// rejects: PARSE-EXPECTED on: port p : ~a.b;
port p : ~a.b;
