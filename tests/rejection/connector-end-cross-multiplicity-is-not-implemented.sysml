// REJECTED BY ABSENCE, not by rule: this text is grammatical. ConnectorEnd opens with an
// optional OwnedCrossMultiplicityMember, and SysML 7.13.5's own example writes one on a
// succession's ends (`first [1] focus then [0..1] shoot;`, receipt 2abd302c). That member
// is unimplemented, so the succession is not recognised and the line is reported.
// Implementing OwnedCrossMultiplicityMember removes this file.
// SysML 8.2.2.13.1  — ConnectorEnd = OwnedCrossMultiplicityMember? ( NAME REFERENCES )?
//                                    OwnedReferenceSubsetting
// SysML 8.2.2.13.3  — SuccessionAsUsage = ... 'first' ConnectorEndMember 'then' ...
// rejects: PARSE-UNEXPECTED on: first [1] focus then [0..1] shoot;
part def Camera {
	first [1] focus then [0..1] shoot;
}
