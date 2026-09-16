// Rule: '/*/' ends in '*/' and is still unterminated — those are the
// opener's own characters.
// KerML 8.2.2.2 — REGULAR_COMMENT = '/*' COMMENT_TEXT '*/'
// The case that catches a terminator check that forgot to skip the opener.
package Vehicle;
/*/