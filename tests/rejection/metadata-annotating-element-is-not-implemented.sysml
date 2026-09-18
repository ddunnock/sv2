// REJECTED BY ABSENCE, not by rule. AnnotatingElement's fourth alternative is the
// one place the two grammars name different productions, and neither is read here.
// SysML 8.2.2.4.1 — AnnotatingElement = Comment | Documentation
//                                     | TextualRepresentation | MetadataUsage
// KerML 8.2.3.3.1 — the same three, then MetadataFeature.
// The fourth alternative is MetadataUsage in SysML by the recorded deviation
// AnnotatingElement (follow_xtext); the clause prints MetadataFeature.
// The other three ARE implemented and are reachable as members, so this case is
// what keeps "an annotating element is a member" from reading as all four.
package P { metadata Safety about Q; }
