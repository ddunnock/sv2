// REJECTED BY RULE. Clients and suppliers are each a [QualifiedName], a reference to an
// Element; a dependency writes no feature chain, where `a.b` would be one.
// SysML 8.2.2.3 — supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
// rejects: PARSE-EXPECTED on: dependency a to b.c;
dependency a to b.c;
