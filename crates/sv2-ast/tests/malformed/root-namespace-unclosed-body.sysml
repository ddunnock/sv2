// Clause: SysML 8.2.2.5.1 -- PackageBody = ';' | '{' PackageBodyElement* '}'
// That is all the specification says here: a package body must close with '}'. It says
// nothing about what to do when one does not.
//
// Recovery (NOT a clause): this parser reads an unclosed body to end of file, so the
// next top-level `package` becomes one of ITS members, not a sibling member of
// RootNamespace. That is this implementation's statement-bounded recovery policy --
// ADR-0022, crates/sv2-syntax/src/parser/namespace.rs. ADR-0025 sanctions pinning it:
// where the grammar admits one child and the tree carries two, the case pins which is
// chosen. Do not cite 8.2.2.5.1 for the nesting outcome.
package Unclosed {
    part def Engine;

package Formed {
    part def Car;
}
