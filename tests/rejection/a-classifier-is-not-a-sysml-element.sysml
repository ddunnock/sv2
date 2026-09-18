// REJECTED BY RULE. Every classifier production of KerML 8.2.4.2 is scoped `kerml`
// in the derived grammar: SysML reaches DefinitionElement from PackageMember, and
// `class` is not one of its alternatives. The mirror of
// part-definition-is-not-a-kerml-element.kerml, and the other half of ADR-0014 —
// neither language silently borrows the other's constructs.
// KerML 8.2.4.2 — Class = TypePrefix 'class' ClassifierDeclaration TypeBody
// SysML 8.2.2.6.1 — PackageMember = MemberPrefix ( DefinitionElement | UsageElement )
// The same line in a .kerml file parses, which is what makes this a rule.
class Vehicle;
