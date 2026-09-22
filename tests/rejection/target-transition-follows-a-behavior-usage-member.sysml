// REJECTED BY RULE. A target transition writes no source: "the source is taken to be the
// closest lexically previous state usage" (7.18.3, receipt 6e6e9493), and the grammar
// admits it only as the suffix of a behaviour usage member, never after an attribute.
// SysML 8.2.2.18.1 — StateBodyItem = …
//                    | SourceSuccessionMember? BehaviorUsageMember
//                      TargetTransitionUsageMember* | …
// rejects: PARSE-UNEXPECTED on: accept S then b;
state def D {
	attribute x;
	accept S then b;
}
