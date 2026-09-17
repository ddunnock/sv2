// Rule: a Usage ends in a UsageBody, which is a DefinitionBody, which is ';' or a
// braced body. A declaration alone is not a usage.
// SysML 8.2.2.6.2 — Usage = UsageDeclaration UsageCompletion
// SysML 8.2.2.6.2 — UsageCompletion = ValuePart? UsageBody
// SysML 8.2.2.6.1 — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
part engine : Engine
