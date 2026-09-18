// REJECTED BY ABSENCE. A ReturnParameterMember is one of CalculationBodyItem's two
// alternatives and this text is well-formed SysML; the member is not implemented.
// SysML 8.2.2.19 — CalculationBodyItem   = ActionBodyItem | ReturnParameterMember
//                  ReturnParameterMember = MemberPrefix? 'return'
//                                          ownedRelatedElement += UsageElement
// `return` is a pinned keyword and a keyword is not a name (KerML 8.2.2.6), so it is
// not read as the trailing expression either — it is reported where it stands.
// Removing this rejection means implementing ReturnParameterMember, which is most of
// what `calc def` needs.
constraint def C {
	return x;
}
