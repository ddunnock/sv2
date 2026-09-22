// REJECTED BY RULE. A send names its sender with `via` BEFORE its receiver with `to`:
// SenderReceiverPart's first alternative is `'via' ... ( 'to' ... )?`, and its second has
// no `via` at all. "values for the three SendAction parameters are given after the action
// declaration part, identified by the keywords send (payload), via (sender) and to
// (receiver)" (7.17.7, receipt db730711).
// SysML 8.2.2.17.4 — SenderReceiverPart = 'via' NodeParameterMember ( 'to' NodeParameterMember )?
//                                       | EmptyParameterMember 'to' NodeParameterMember
// rejects: PARSE-EXPECTED on: send s to b via p;
action def A {
	send s to b via p;
}
