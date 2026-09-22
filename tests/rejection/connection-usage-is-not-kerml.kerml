// REJECTED BY RULE. ConnectionUsage is stated by SysML 8.2.2.13.1 alone and is reachable
// only from SysML's start symbol (ADR-0014). KerML's connector is `connector` (KerML
// 8.2.5.5.1), and no KerML production writes the terminal `connect`. KerML does not
// reserve the word either (8.2.2.6), so there `connect` is a NAME and the line is also a
// Feature's declaration that `a to b` cannot continue; the parser reports it as a keyword
// only through its single keyword table, pending decision [keyword-table-per-language].
// Either way it is not a ConnectionUsage, which is the rule this file holds.
// rejects: PARSE-UNEXPECTED on: connect a to b;
package P {
	connect a to b;
}
