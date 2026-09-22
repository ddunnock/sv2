// REJECTED BY RULE. `to` and the supplier list are not optional: Dependency::supplier is
// 1..* (KerML 8.3.2.2.2, receipt ec1e3424).
// SysML 8.2.2.3 — DependencyDeclaration = … client ( ',' client )* 'to'
//                     supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
// rejects: PARSE-EXPECTED on: dependency a;
dependency a;
