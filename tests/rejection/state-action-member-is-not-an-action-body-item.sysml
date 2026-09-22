// REJECTED BY RULE. Entry, do and exit actions are StateBodyItems: StateSubactionMembership
// relates a state to its subactions, and no action body item names them.
// SysML 8.2.2.18.1 — StateBodyItem = … | EntryActionMember EntryTransitionMember*
//                                    | DoActionMember | ExitActionMember
// SysML 8.2.2.17.1 — ActionBodyItem has none of the three.
// rejects: PARSE-UNEXPECTED on: entry;
action def A {
	entry;
}
