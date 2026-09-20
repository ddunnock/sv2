// REJECTED BY ABSENCE, not by rule. `else A3;` is well-formed SysML: the third
// ActionTargetSuccession alternative, the unguarded branch after the guarded ones, written
// in vendor/corpus/sysml/src/examples/Simple Tests/DecisionTest.sysml:7. The other two
// alternatives are implemented; this one is not, so the `else` is reported where it
// stands. Implementing it removes this file.
// SysML 8.2.2.17.8  — ActionTargetSuccession = ( TargetSuccession
//                                             | GuardedTargetSuccession
//                                             | DefaultTargetSuccession ) UsageBody
//                     DefaultTargetSuccession = 'else' TransitionSuccessionMember
action def A {
	first start;
	then decide;
	if x == 1 then A1;
	else A2;
}
