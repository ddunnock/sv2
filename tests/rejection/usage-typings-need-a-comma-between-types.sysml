// Rule: a second type in a Typings is reached only through ','.
// SysML 8.2.2.6.5 — Typings = TypedBy ( ',' ownedRelationship += FeatureTyping )*
// Two names juxtaposed are not two types; the second is not a FeatureSpecialization
// and so cannot continue the FeatureSpecializationPart either.
// KerML 8.2.4.3.1 — FeatureSpecializationPart = FeatureSpecialization+ ...
part engine : Engine Motor;
