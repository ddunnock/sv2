// REJECTED BY RULE. AttributeDefinition takes a DefinitionPrefix and only an
// OccurrenceDefinitionPrefix carries `individual`, so an attribute definition
// cannot be individual. The definition-level twin of
// attribute-usage-is-not-an-occurrence.sysml.
// SysML 8.2.2.7   — AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
// SysML 8.2.2.6.1 — DefinitionPrefix = BasicDefinitionPrefix?
//                                      DefinitionExtensionKeyword*
// SysML 8.2.2.9.1 — OccurrenceDefinitionPrefix adds
//                   ( 'individual' EmptyMultiplicityMember )?
// `individual part def V;` parses, which is what makes this a rule and not an absence.
individual attribute def A;
