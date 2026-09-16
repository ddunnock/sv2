// Rule: a braced RelationshipBody holds OwnedAnnotations, nothing else.
// SysML 8.2.2.2 — RelationshipBody = ';' | '{' OwnedAnnotation* '}'
// OwnedAnnotation is unimplemented, so this parser must report the
// contents rather than accept text it cannot read as an annotation.
// Remove this case when OwnedAnnotation lands; it is a rejection by
// absence, not by rule, and the header says so on purpose.
public import A::* { doc /* an annotation */ }