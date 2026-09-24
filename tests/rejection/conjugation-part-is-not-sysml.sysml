// REJECTED BY RULE. ConjugationPart is KerML's (KerML 8.2.4.1.1): no SysML definition
// declaration reaches it, and a .sysml file is read against the SysML grammar alone
// (ADR-0014). SysML's one conjugation is ConjugatedPortTyping's `~` after `:`.
// SysML 8.2.2.6.1 — DefinitionDeclaration = Identification SubclassificationPart?
// rejects: PARSE-EXPECTED on: part def B conjugates A;
part def B conjugates A;
