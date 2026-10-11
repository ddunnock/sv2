// Rule: an unclosed PackageBody is read to end of file; the next top-level `package`
// becomes one of ITS members, not a sibling member of RootNamespace.
// SysML 8.2.2.5.1 -- PackageBody = ';' | '{' PackageBodyElement* '}'
package Unclosed {
    part def Engine;

package Formed {
    part def Car;
}
