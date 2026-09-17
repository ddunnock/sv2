// Rule: References carries exactly one target and no repetition, unlike Subsettings
// and Redefinitions, so a ',' after the target continues nothing.
// SysML 8.2.2.6.5 — References = REFERENCES ownedRelationship += OwnedReferenceSubsetting
// SysML 8.2.2.6.5 — Subsettings = Subsets ( ',' OwnedSubsetting )*    [contrast]
part p ::> a, b;
