// REJECTED BY RULE. `parallel` marks a state whose substates run concurrently, and stands
// "just before the body part" (7.18.2, receipt 42b13f63) — before braces only.
// SysML 8.2.2.18.1 — StateDefBody = ';' | ( isParallel ?= 'parallel' )? '{' StateBodyItem* '}'
// rejects: PARSE-EXPECTED on: state def D parallel;
state def D parallel;
