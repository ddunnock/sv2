// Clause: KerML 8.2.3.4.1 -- QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
//
// A `::` separates two segment names. `A::::B` writes a separator with no name between
// the two, so it is not a qualified name of three segments; it is not a qualified name
// past `A` at all.
//
// The second fixture for this kind, and deliberately so. The first
// (global-qualifier-names-nothing.sysml) pins ABSENCE: nothing was written, so nothing
// is reported. This one pins ATTRIBUTION, which is the other way a token accessor goes
// wrong: something WAS written, and it is not this node's.
//
// Recovery (NOT a clause, and NOT an ADR): `qualified_name`
// (crates/sv2-syntax/src/parser/import.rs) loops
// `while self.at(ColonColon) && self.name_follows_separator()`, and the comment there
// states the reason for the second condition -- a `::` is only part of the name when a
// NAME follows it. At the first `::` the next token is another `::`, so the loop does not
// run and the name ENDS AT `A`. Everything from there is no longer the qualified name's
// business, and the enclosing relationship reports it ("expected `;` or `{` to close the
// relationship, found `::`") and recovers it into one Error node, the closing `;`
// included:
//
//   AliasMember "alias X for A"
//     ...
//     QualifiedName "A"        <- ONE segment, though the text writes three names
//   Error "::::B;"             <- `B` is here, and it is not a segment of anything
//
// So `name()` must report `A` and must NOT report `B`, although `B` is the last name a
// reader sees and is what an accessor reading the node's TEXT, or scanning past the
// node's own tokens, would hand back. `B` is in the tree -- the parse stays lossless
// (ADR-0004) -- just not in this node, and "the last name written" and "the final
// segment of this qualified name" are different questions whose answers differ here.
//
// One diagnostic covers the whole lost run, so the count of diagnostics is not a measure
// of how many segments were lost.
//
// Do not cite 8.2.3.4.1 for the recovery outcome, and do not cite ADR-0022.
alias X for A::::B;
