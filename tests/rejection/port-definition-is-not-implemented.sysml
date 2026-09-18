// REJECTED BY ABSENCE, not by rule. PortDefinition shares the `'def' Definition`
// spine of the eight definitions this parser reads, and then carries one more part
// that none of them has.
// SysML 8.2.2.12 — PortDefinition = DefinitionPrefix 'port' 'def' Definition
//                                   ConjugatedPortDefinitionMember
// ConjugatedPortDefinitionMember is unimplemented, so PortDefinition is not in the
// table: reading it with the shared spine would accept a port definition while
// silently dropping the conjugated port every one of them declares.
port def P;
