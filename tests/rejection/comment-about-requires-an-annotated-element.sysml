// Rule: 'about' is followed by at least one Annotation, a qualified name.
// SysML 8.2.2.4.2 — Comment = ( 'comment' Identification
//   ( 'about' ownedRelationship += Annotation ( ',' ownedRelationship += Annotation )* )? )?
//   ( 'locale' locale = STRING_VALUE )? body = REGULAR_COMMENT
// SysML 8.2.2.4.1 — Annotation = annotatedElement = [QualifiedName]
public import A::* { comment about /* nothing is annotated */ }
