// REJECTED BY ABSENCE, not by rule: this text is grammatical. `succession flow` opens a
// SuccessionFlow, which is unimplemented; Succession's recogniser declines it rather
// than read a succession whose declaration names `flow`. The corpus writes it:
// vendor/corpus/kerml/src/examples/Behavior Examples/TakePicture.kerml line 14.
// Implementing SuccessionFlow removes this file.
// KerML 8.2.5.9.2 — SuccessionFlow = FeaturePrefix 'succession' 'flow' FlowDeclaration TypeBody
// rejects: PARSE-UNEXPECTED on: succession flow of Exposure from a.x to b.y;
class C {
	succession flow of Exposure from a.x to b.y;
}
