// Rule: a definition carries at most one BasicDefinitionPrefix, 'abstract' or 'variation'.
// SysML 8.2.2.9.1 — OccurrenceDefinitionPrefix = BasicDefinitionPrefix?
//   ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
//   DefinitionExtensionKeyword*
// SysML 8.2.2.6.1 — BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?= 'variation'
abstract variation part def Vehicle;
