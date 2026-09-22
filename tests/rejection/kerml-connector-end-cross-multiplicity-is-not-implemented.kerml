// REJECTED BY ABSENCE, not by rule: this text is grammatical. ConnectorEnd opens with an
// optional OwnedCrossMultiplicityMember, which is unimplemented, so the `[1]` is not read.
// The KerML corpus writes it on most of its successions, e.g. vendor/corpus/kerml/src/
// examples/KerML Spec Annex A Examples/A-3-6-Sequences.kerml line 10. SysML reads the
// same member from the ConnectionUsage commit on; KerML's does not, because it reaches
// OwnedMultiplicity@kerml = OwnedMultiplicityRange, which is unimplemented.
// Implementing OwnedCrossMultiplicityMember removes this file.
// KerML 8.2.5.5.1 — ConnectorEnd = OwnedCrossMultiplicityMember? ( NAME REFERENCES )?
//                                  OwnedReferenceSubsetting
// rejects: PARSE-EXPECTED on: succession p_before_d first [1] paint then [1] dry;
class C {
	succession p_before_d first [1] paint then [1] dry;
}
