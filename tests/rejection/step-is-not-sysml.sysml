// REJECTED BY RULE. `step` is KerML's (KerML 8.2.5.6.2): SysML writes its steps as
// action usages (SysML 8.2.2.17), no SysML production reaches Step, and a .sysml file
// is read against the SysML grammar alone (ADR-0014).
// SysML 8.2.2.17.1 — ActionBody = ';' | '{' ActionBodyItem* '}'
// rejects: PARSE-EXPECTED on: step paint : Paint [1];
action def Painting {
	step paint : Paint [1];
}
