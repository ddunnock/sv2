// Clause: SysML 8.2.2.2 -- RelationshipBody = ';' | '{' ( ownedRelationship +=
// OwnedAnnotation )* '}'
//
// The second alternative is a MATCHED PAIR. A `{` with no `}` is neither alternative, and
// the clause does not describe what that leaves.
//
// This file exists because the corpus cannot settle the question. Of the 857
// RelationshipBody nodes in this repository's 610 trees, 827 are a bare `;`, 16 are braced
// WITH BOTH BRACES, and 14 are zero width -- so no pinned tree holds a body whose last
// token is a `{`. Mutating terminator() to accept whatever token comes last, rather than
// only a `;` or a `}`, therefore survived the entire suite: with no unclosed body
// anywhere, "the last token" and "the last token if it closes the body" agree everywhere.
// A hand-written case is legitimate for exactly that (.claude/rules/tests.md), and its
// expectation comes from the production above, never from what the accessor printed.
//
// Recovery (NOT a clause, and NOT an ADR): `sysml_relationship_body`
// (crates/sv2-syntax/src/parser/annotation.rs) takes the `{` and calls
// `owned_annotations`, and it is THERE that the run ends with `expect(RBrace)` -- which
// REPORTS without consuming when the brace never arrives. Naming the entry point rather
// than the site is the mistake this header avoids: the `expect` is in `owned_annotations`,
// reached through that call, and the KerML path reaches its own by the same shape. So the node keeps its `{` and gains nothing
// else:
//
//   Import "public import A::* {"
//     ...
//     RelationshipBody "{"
//       tok LBrace "{"        <- a token, and NOT a terminator
//                             <- no `}`, and no children
//
// What that pins, and what the 14 zero-width bodies cannot:
//
//  1. terminator() is None although the body HOLDS A TOKEN and is not zero width. The two
//     readings of "unterminated" come apart here, and only here: the zero-width bodies are
//     empty AND unterminated, so a terminator() that answered from emptiness would be
//     right about all 14 of them and wrong about this one.
//  2. It is found by POSITION, not by searching for a closer anywhere in the node. Taking
//     the last token and then asking whether it closes the body is the only form that
//     answers None here, which is why the accessor is written that way round.
//  3. elements() is empty, and that says nothing about termination: `{ }` in the corpus is
//     also empty and IS terminated. Emptiness and closure are independent.
//
// ADR-0004 supports only the shape -- the `{` still reaches the tree, so this round-trips.
// Do not cite 8.2.2.2 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
public import A::* {
