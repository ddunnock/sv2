// REJECTED BY RULE. A state's send action names its payload. SendNode's parameter group is
// optional, and `send via p;` is one in an action body; the state and transition forms read
// SendNodeDeclaration instead, whose NodeParameterMember is not.
// SysML 8.2.2.18.1 — StateSendActionUsage = SendNodeDeclaration ActionBody
// SysML 8.2.2.17.4 — SendNodeDeclaration = ActionNodeUsageDeclaration? 'send'
//                                          NodeParameterMember SenderReceiverPart?
// rejects: PARSE-EXPECTED on: entry send via p;
state def D {
	entry send via p;
}
