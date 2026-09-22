// REJECTED BY RULE. A dependency that is given a short name or a name must write `from`
// before its clients: "If no short name or name is given for the dependency, then the
// keyword from may be omitted" (7.3.2, receipt 65989bd2) — and only then.
// SysML 8.2.2.3 — DependencyDeclaration = ( Identification 'from' )?
//                     client ( ',' client )* 'to' supplier ( ',' supplier )*
// rejects: PARSE-EXPECTED on: dependency <u> a to b;
dependency <u> a to b;
