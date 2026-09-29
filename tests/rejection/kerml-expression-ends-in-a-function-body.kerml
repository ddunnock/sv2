// REJECTED BY RULE. An Expression ends in a FunctionBody, `;` or braced; with neither,
// the declaration runs into whatever follows.
// KerML 8.2.5.7.2 — Expression   = FeaturePrefix 'expr' FeatureDeclaration ValuePart?
//                                  FunctionBody
// KerML 8.2.5.7.1 — FunctionBody = ';' | '{' FunctionBodyPart '}'
// rejects: PARSE-EXPECTED on: package Expressions { expr totalMass : TotalMass }
package Expressions { expr totalMass : TotalMass }
