// REJECTED BY RULE. The `succession` keyword and its UsageDeclaration are the optional
// part of SuccessionAsUsage; `'first' ConnectorEndMember 'then' ConnectorEndMember` is not.
// A succession declared with no ends is no production -- the KerML Succession, which does
// allow it (8.2.5.5.3), is not reachable from a .sysml file (ADR-0014).
// SysML 8.2.2.13.3  — SuccessionAsUsage = UsagePrefix ( 'succession' UsageDeclaration )?
//                                         'first' ConnectorEndMember
//                                         'then' ConnectorEndMember UsageBody
// rejects: PARSE-UNEXPECTED on: succession s;
part def P {
	succession s;
}
