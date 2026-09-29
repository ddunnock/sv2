// REJECTED BY RULE. A FeatureTyping names its typed feature by qualified name alone.
// Its type is a GeneralType and may be a feature chain; the typed feature may not.
// KerML 8.2.4.3.2 — FeatureTyping = ( 'specialization' Identification )?
//                       'typing' typedFeature = [QualifiedName]
//                       TYPED_BY GeneralType RelationshipBody
// rejects: PARSE-EXPECTED on: typing customer.address typed by Address;
package Features {
	typing customer.address typed by Address;
}
