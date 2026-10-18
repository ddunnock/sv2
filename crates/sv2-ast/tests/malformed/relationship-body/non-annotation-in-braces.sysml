// Clause: SysML 8.2.2.2 -- RelationshipBody = ';' | '{' ( ownedRelationship +=
// OwnedAnnotation )* '}'
//
// The braces admit OwnedAnnotation and NOTHING ELSE in SysML. (KerML 8.2.3.1 is wider:
// its RelationshipOwnedElement is `OwnedRelatedElement | OwnedAnnotation`, and
// OwnedRelatedElement is `NonFeatureElement | FeatureElement` -- which is why one
// RelationshipBody in the pinned corpus owns a Feature, and why elements() does not
// filter to annotations. Neither alternation builds a node, so what lands in the braces
// is the concrete element.)
//
// A part definition is not an annotation in either language. This fixture writes one
// inside the braces, after a comment that IS an annotation, and the body closes normally.
//
// Recovery (NOT a clause, and NOT an ADR): the braced run reads an annotation when one
// starts and otherwise recovers ONE TOKEN AT A TIME, reporting each. The site is
// `owned_annotations`, which `sysml_relationship_body` calls once it has taken the `{`,
// and which recovers with `error_token` per token
// (crates/sv2-syntax/src/parser/annotation.rs). `sysml_relationship_body`'s own comment
// states the reason: accepting the run silently "would report an annotation this parser
// cannot read as one it understood". So `part def X;` becomes FOUR Error nodes, one per token,
// each with its own diagnostic:
//
//   RelationshipBody "{ /* a */ part def X; }"
//     tok LBrace
//     OwnedAnnotation "/* a */"   <- the one real annotation
//     Error "part"                <- four fragments of something that is not one
//     Error "def"
//     Error "X"
//     Error ";"
//     tok RBrace                  <- and the body still CLOSED
//
// Three things make that the case worth pinning:
//
//  1. elements() yields FIVE children for ONE annotation. A caller counting them to mean
//     "how many annotations" is wrong by four, and nothing about the shape of the node
//     says so -- the body is braced, closed, and non-empty, which is exactly what a
//     healthy annotated relationship looks like. The kinds are the only structural
//     signal, and Parse::errors() is the only one that says something failed.
//  2. CLOSED IS NOT HEALTHY. terminator() returns the `}`, because the body did close.
//     An accessor or caller that read "terminated" as "well formed" would pass this.
//     The opposite case -- a body that never closed -- is the 14 zero-width bodies the
//     corpus already contains, where terminator() is None.
//  3. One token per diagnostic, so FOUR diagnostics for one stray statement. The count of
//     diagnostics is not the count of mistakes, in the opposite direction from
//     tests/malformed/member-prefix/visibilities-lost-to-recovery.sysml, where two lost
//     words share one diagnostic.
//
// ADR-0002 is why the Errors are returned rather than filtered: elements with failed
// constraints still enter the IR carrying their diagnostics, and validity gates writes,
// never reads. ADR-0004 supports only the shape -- every token reaches the tree, so this
// round-trips. Do not cite 8.2.2.2 for the recovery outcome, and do not cite ADR-0022,
// which is about deviation diagnostics and says nothing about recovery.
public import A::* {
	/* a */
	part def X;
}
