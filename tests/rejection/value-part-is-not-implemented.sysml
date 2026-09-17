// REJECTION BY ABSENCE, not by rule. A usage may carry a value, and the corpus
// writes them. FeatureValue reaches OwnedExpression and so an expression parser,
// which sv2-syntax does not have yet.
// SysML 8.2.2.6.2 — UsageCompletion = ValuePart? UsageBody
// SysML 8.2.2.6.2 — ValuePart = ownedRelationship += FeatureValue
// SysML 8.2.2.6.2 — FeatureValue = ( '=' | isInitial ?= ':='
//                                  | isDefault ?= 'default' ( '=' | isInitial ?= ':=' )? )
//                                  ownedRelatedElement += OwnedExpression
// Remove this case when ValuePart lands.
part engine : Engine = x;
