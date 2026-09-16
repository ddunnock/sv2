// Rule: an Import ends in a RelationshipBody.
// SysML 8.2.2.5.1 — Import = ... ImportDeclaration RelationshipBody
// SysML 8.2.2.2   — RelationshipBody = ';' | '{' OwnedAnnotation* '}'
// Neither form is optional, so a bare declaration is not an Import.
public import A::*