// REJECTED BY RULE. `/* ... */` is REGULAR_COMMENT, a token, not a note: it is the body
// of a Comment, Documentation or TextualRepresentation, and a bare one is a Comment, which
// stands only where a member may. Between a declaration's tokens it is nothing. The notes
// are `//* ... */` and `// ...`, which the Pilot hides with whitespace; it hides no
// REGULAR_COMMENT (KerMLExpressions.xtext:29).
// KerML 8.2.2.2 — REGULAR_COMMENT = '/*' COMMENT_TEXT '*/'
// SysML 8.2.2.4.2 — Comment = ( 'comment' Identification ... )? ( 'locale' ... )?
//                             body = REGULAR_COMMENT
// rejects: PARSE-UNEXPECTED on: 	part engine : /* the one we chose */ Engine;
part def Vehicle {
	part engine : /* the one we chose */ Engine;
}
