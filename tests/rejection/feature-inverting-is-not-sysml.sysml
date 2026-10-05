// REJECTED BY RULE. FeatureInverting is a KerML NonFeatureElement. SysML's
// PackageBodyElement reaches no such production, and a .sysml file is parsed from
// SysML's start symbol alone (ADR-0014).
// KerML 8.2.3.4.3 — NonFeatureElement = ... | FeatureInverting | ...
// KerML 8.2.4.3.6 — FeatureInverting  = ( 'inverting' Identification? )? 'inverse' ...
// rejects: PARSE-EXPECTED on: inverse Person::parents of Person::children;
package Family {
	inverse Person::parents of Person::children;
}
