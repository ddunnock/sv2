// REJECTED BY RULE. KerML's Succession always writes its keyword. `first a then b;` with
// no `succession` is SysML's SuccessionAsUsage, whose keyword "may be omitted" (SysML
// 7.13.5); no KerML production opens on `first`, and a .kerml file does not reach the
// SysML one (ADR-0014).
// KerML 8.2.5.5.3 — Succession = FeaturePrefix 'succession' SuccessionDeclaration TypeBody
// rejects: PARSE-UNEXPECTED on: first a then b;
class C {
	first a then b;
}
