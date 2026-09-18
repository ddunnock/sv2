// REJECTED BY RULE, and the rule is the difference between this declaration and an
// action's.
// SysML 8.2.2.17.2 — PerformActionUsageDeclaration =
//                        ( ownedRelationship += OwnedReferenceSubsetting
//                          FeatureSpecializationPart?
//                        | 'action' UsageDeclaration ) ValuePart?
//                    ActionUsageDeclaration = UsageDeclaration ValuePart?
// An ActionUsage may name nothing — UsageDeclaration is Identification and a
// FeatureSpecializationPart, both optional, so `action;` is an anonymous action and
// parses. A PerformActionUsage may NOT: neither alternative derives the empty string,
// because the first needs a referenced feature and the second needs the `action`
// keyword. `perform` names something by definition; a perform that performs nothing is
// not a shorter form, it is not the production.
action def B {
	perform;
}
