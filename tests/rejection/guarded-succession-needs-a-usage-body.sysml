// REJECTED BY RULE. GuardedSuccession ends in UsageBody -- `;` or a braced body -- and it
// is the last part of the production, not an optional one. The member is recognised by its
// guard, so it IS read and UsageBody reports the missing body itself.
// SysML 8.2.2.17.8  — GuardedSuccession = ( 'succession' UsageDeclaration )?
//                                         'first' FeatureChainMember GuardExpressionMember
//                                         'then' TransitionSuccessionMember UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
action def A {
	first a if x then b
}
