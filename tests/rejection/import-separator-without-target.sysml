// Rule: '::' must be followed by a NAME, '*' or '**'.
// KerML 8.2.3.4.1 — QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
// SysML 8.2.2.5.1 — MembershipImport = [QualifiedName] ( '::' '**' )?
//                   NamespaceImport  = [QualifiedName] '::' '*' ...
// A trailing '::' belongs to no alternative.
public import A::;