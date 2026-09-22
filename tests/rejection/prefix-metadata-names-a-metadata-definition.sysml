// REJECTED BY RULE. A `#` is followed by the name of a metadata definition: "the (possibly
// qualified) name (or short name) of a metadata definition (or KerML metaclass) preceded
// by the symbol #" (7.27.4, receipt 0f2c5bd1). A bare `#` names nothing.
// SysML 8.2.2.27 — PrefixMetadataMember = '#' ownedRelatedElement = PrefixMetadataUsage
//                  PrefixMetadataUsage = ownedRelationship += OwnedFeatureTyping
// rejects: PARSE-UNEXPECTED on: # part engine;
part def Vehicle {
	# part engine;
}
