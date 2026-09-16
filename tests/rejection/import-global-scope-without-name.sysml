// Rule: the global scope qualifier prefixes a name, it is not one.
// KerML 8.2.3.4.1 — QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
// The final NAME is not optional.
public import $::;