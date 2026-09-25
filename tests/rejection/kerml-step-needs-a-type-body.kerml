// REJECTED BY RULE. A step ends in a TypeBody, `;` or a braced body, as every KerML
// feature element does: the declaration alone does not close it.
// KerML 8.2.5.6.2 — Step = FeaturePrefix 'step' FeatureDeclaration ValuePart? TypeBody
// KerML 8.2.4.1.1 — TypeBody = ';' | '{' TypeBodyElement* '}'
// rejects: PARSE-EXPECTED on: }
behavior Paint {
	step paint : Paint [1]
}
