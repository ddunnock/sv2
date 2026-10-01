// REJECTED BY RULE. `/* ... */` is REGULAR_COMMENT, a token, not a note: a bare one is a
// Comment, a MemberElement, and stands only where a member may. Between a declaration's
// tokens it is nothing; the Pilot hides only WS, ML_NOTE and SL_NOTE
// (KerMLExpressions.xtext:29).
// KerML 8.2.2.2 — REGULAR_COMMENT = '/*' COMMENT_TEXT '*/'
// KerML 8.2.3.3.2 — Comment = ( 'comment' Identification ... )? ( 'locale' ... )?
//                             body = REGULAR_COMMENT
// rejects: PARSE-UNEXPECTED on: 	feature engine : /* the one we chose */ Engine;
classifier Vehicle {
	feature engine : /* the one we chose */ Engine;
}
