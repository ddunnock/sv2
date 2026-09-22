// REJECTED BY RULE. A transition's send effect takes a braced body or none, never `;`:
// the `then` after the effect is what ends it, as in `do send s to p then S1;`
// (examples/Simple Tests/StateTest.sysml:36-37). AS ADJUDICATED: 7.18.3's own OnOff5
// example writes `do send new TimeoutSignal() via commPort;` before its `then`, and
// deviations.json TransitionUsage (conflict, follow_spec, SYSML21-450) records that the
// production, not the example, is followed.
// SysML 8.2.2.18.3 — TransitionSendActionUsage = SendNodeDeclaration ( '{' ActionBodyItem* '}' )?
// rejects: PARSE-EXPECTED on: do send s to p;
state def D {
	transition t first a
		do send s to p;
		then b;
}
