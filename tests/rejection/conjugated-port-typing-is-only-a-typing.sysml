// REJECTED BY RULE. `~` conjugates a TYPING only: ConjugatedPortTyping is an alternative
// of FeatureTyping and of no subsetting, redefinition or subclassification, whose targets
// are plain references.
// SysML 8.2.2.6.5 — FeatureTyping = OwnedFeatureTyping | ConjugatedPortTyping
//                   OwnedSubsetting = [QualifiedName] | OwnedFeatureChain
// rejects: PARSE-EXPECTED on: port p :> ~q;
port p :> ~q;
