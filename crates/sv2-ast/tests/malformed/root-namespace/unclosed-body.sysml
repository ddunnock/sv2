// Clause: SysML 8.2.2.5.1 -- PackageBody = ';' | '{' PackageBodyElement* '}'
// That is all the specification says here: a package body must close with '}'. It says
// nothing about what to do when one does not.
//
// Recovery (NOT a clause, and NOT an ADR): this parser reads an unclosed body to end of
// file, so the next top-level `package` becomes one of ITS members, not a sibling member
// of RootNamespace. The only source is the code -- `body_elements` in
// crates/sv2-syntax/src/parser/namespace.rs loops while not at_end() and not at the
// terminator, so a body missing its '}' consumes the rest of the file. ADR-0004 supports
// only the shape (an error node is just another node, so the partial tree survives), not
// this outcome. ADR-0025 sanctions pinning it: where the grammar admits one child and the
// tree carries two, the case pins which is chosen.
//
// Do not cite 8.2.2.5.1 for the nesting, and do not cite ADR-0022 -- that is about
// deviation diagnostics and strict conformance, and says nothing about recovery.
package Unclosed {
    part def Engine;

package Formed {
    part def Car;
}
