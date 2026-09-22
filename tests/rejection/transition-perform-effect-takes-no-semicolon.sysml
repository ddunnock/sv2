// REJECTED BY RULE. A transition's perform effect takes a braced body or none: the `then`
// after it ends it, and TransitionPerformActionUsage has no `;` alternative, unlike a
// state's perform action, which takes an ActionBody. 7.18.3's OnOff4 and OnOff5 write
// `do action powerUp : PowerUp;` before `then`; that is filed as SYSML21-450 and the
// grammar is followed (deviation TransitionUsage).
// SysML 8.2.2.18.3 — TransitionPerformActionUsage =
//                      PerformActionUsageDeclaration ( '{' ActionBodyItem* '}' )?
// rejects: PARSE-EXPECTED on: transition t first a do action x; then b;
state def D {
	state a;
	transition t first a do action x; then b;
}
