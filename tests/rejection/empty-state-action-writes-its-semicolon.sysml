// REJECTED BY RULE. An empty entry, do or exit action is written by its `;`: "If the
// keyword is immediately followed by a semicolon ;, then they are empty actions" (7.18.2,
// receipt 42b13f63). Unlike a transition's empty effect, a state's has no other end.
// SysML 8.2.2.18.1 — StateActionUsage = EmptyActionUsage ';' | StatePerformActionUsage | …
// rejects: PARSE-EXPECTED on: entry }
state def D {
	entry }
