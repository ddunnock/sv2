// Rule: PackageBody is not optional.
// SysML 8.2.2.5.1 — Package = PrefixMetadataMember* PackageDeclaration PackageBody
//                   PackageBody = ';' | '{' PackageBodyElement* '}'
// A declaration with neither form is not a Package.
package Vehicle