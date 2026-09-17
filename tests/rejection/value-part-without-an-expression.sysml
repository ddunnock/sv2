// REJECTED BY RULE. FeatureValue owns an OwnedExpression; the `=` does not stand
// alone. `attribute x = 5;` parses, so this is the rule and not an absence.
// SysML 8.2.2.6.2 — FeatureValue = ( '=' | isInitial ?= ':='
//                                  | isDefault ?= 'default' ( '=' | ':=' )? )
//                                  ownedRelatedElement += OwnedExpression
attribute x = ;
