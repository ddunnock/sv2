// Rule: a braced RelationshipBody owns OwnedAnnotations and nothing else.
// SysML 8.2.2.2   — RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
// SysML 8.2.2.4.1 — OwnedAnnotation = ownedRelatedElement += AnnotatingElement
// A package is a DefinitionElement, not an AnnotatingElement.
public import A::* { package P; }
