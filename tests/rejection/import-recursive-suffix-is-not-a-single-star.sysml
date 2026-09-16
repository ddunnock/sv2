// Rule: the recursive suffix is '**', never '*'.
// SysML 8.2.2.5.1 — NamespaceImport =
//   importedNamespace = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
// A second '::' after the '*' must reach '**'.
public import A::*::*;