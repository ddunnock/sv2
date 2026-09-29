// REJECTED BY RULE. A standalone TypeFeaturing writes `by` before its featuring type,
// not `featured by`, which is the keyword pair of a feature's owned TypeFeaturingPart
// (KerML 8.2.4.3.1). KerML 7.3.4.8's examples write this line as below; deviations.json
// entry TypeFeaturing records that as an editing error in the examples (conflict,
// follow_spec): the clause, the Pilot and the corpus all write bare `by`.
// KerML 8.2.4.3.7 — TypeFeaturing = 'featuring' ( Identification 'of' )?
//                       featureOfType = [QualifiedName]
//                       'by' featuringType = [QualifiedName]
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: featuring engine_by_Vehicle of engine featured by Vehicle;
package Vehicles {
	classifier Vehicle;
	feature engine;
	featuring engine_by_Vehicle of engine featured by Vehicle;
}
