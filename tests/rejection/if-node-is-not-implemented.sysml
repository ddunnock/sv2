// REJECTED BY ABSENCE, not by rule. `if i < 0 { … }` is well-formed SysML: an IfNode,
// one of ActionNode's eight alternatives, of which only ControlNode is implemented.
// It is what makes the guarded succession's recogniser look for the `then`: both open on
// `if`, and only GuardedTargetSuccession writes a `then` before its body. Implementing
// IfNode removes this file, and with it the ExpressionParameterMember,
// ActionBodyParameterMember and IfNodeParameterMember it owns.
// SysML 8.2.2.17.2  — ActionNode = ControlNode | SendNode | AcceptNode | AssignmentNode
//                                | TerminateNode | IfNode | WhileLoopNode | ForLoopNode
// SysML 8.2.2.17.7  — IfNode = ActionNodePrefix 'if' ExpressionParameterMember
//                              ActionBodyParameterMember
//                              ( 'else' ( ActionBodyParameterMember
//                                       | IfNodeParameterMember ) )?
// rejects: PARSE-UNEXPECTED on: if i < 0 { }
action def A {
	action a;
	if i < 0 { }
}
