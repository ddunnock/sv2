// REJECTED BY RULE. RequirementBody is not optional, and neither of its two forms is
// written here.
// SysML 8.2.2.21.1 — RequirementDefinition = OccurrenceDefinitionPrefix 'requirement'
//                                            'def' DefinitionDeclaration RequirementBody
//                    RequirementBody       = ';' | '{' RequirementBodyItem* '}'
// The sibling of tests/rejection/part-definition-missing-definition-body.sysml, one
// production along: a declaration that ends without a body ends nothing.
requirement def R
