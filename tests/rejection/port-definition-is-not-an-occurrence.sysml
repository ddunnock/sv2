// REJECTED BY RULE. PortDefinition takes a DefinitionPrefix, and only an
// OccurrenceDefinitionPrefix carries `individual`, so a port definition cannot be
// individual. The same rule attribute-definition-is-not-an-occurrence.sysml states,
// and worth holding separately because PortDefinition is NOT on the shared spine:
// it is written out on its own, so it could acquire the wrong prefix without the
// eight definitions noticing.
// SysML 8.2.2.12  — PortDefinition = DefinitionPrefix 'port' 'def' Definition
//                                    ConjugatedPortDefinitionMember
// SysML 8.2.2.6.1 — DefinitionPrefix = BasicDefinitionPrefix?
//                                      DefinitionExtensionKeyword*
// SysML 8.2.2.9.1 — OccurrenceDefinitionPrefix adds
//                   ( 'individual' EmptyMultiplicityMember )?
// `individual part def V;` parses, which is what makes this a rule and not an absence.
individual port def P;
