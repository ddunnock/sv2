// REJECTED BY RULE. A dependency has one or more clients: the list is
// `client ( ',' client )*`, and Dependency::client is 1..* (KerML 8.3.2.2.2, receipt
// ec1e3424). `from` followed at once by `to` names none.
// SysML 8.2.2.3 — DependencyDeclaration = ( Identification 'from' )?
//                     client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to' …
// rejects: PARSE-EXPECTED on: dependency from to b;
dependency from to b;
