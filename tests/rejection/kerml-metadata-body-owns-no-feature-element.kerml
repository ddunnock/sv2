// REJECTED BY RULE. A MetadataBody owns NonFeatureMembers, AliasMembers, Imports and
// MetadataBodyFeatureMembers. Its features are redefinitions of the metaclass's
// features; a FeatureElement such as a Step is not among its elements.
// KerML 8.2.5.12 — MetadataBody        = ';' | '{' MetadataBodyElement* '}'
//                  MetadataBodyElement = NonFeatureMember | MetadataBodyFeatureMember
//                                      | AliasMember | Import
// rejects: PARSE-UNEXPECTED on: step review;
package Annotations {
	metadata ApprovalAnnotation about Design {
		approved = true;
		step review;
	}
}
