// REJECTED BY RULE. The `then` shorthand after a state action names the state "entered on
// completion of the entry action" (7.18.2, receipt 42b13f63): it follows an ENTRY action
// only, and not a do or an exit action.
// SysML 8.2.2.18.1 — StateBodyItem = … | EntryActionMember EntryTransitionMember*
//                                    | DoActionMember | ExitActionMember
// rejects: PARSE-UNEXPECTED on: then b;
state def D {
	do a;
	then b;
}
