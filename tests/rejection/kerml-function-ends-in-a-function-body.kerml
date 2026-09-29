// REJECTED BY RULE. A Function ends in a FunctionBody, `;` or braced, and a function
// body's result expression takes no semicolon.
// KerML 8.2.5.7.1 — Function         = TypePrefix 'function' ClassifierDeclaration
//                                      FunctionBody
//                   FunctionBody     = ';' | '{' FunctionBodyPart '}'
//                   FunctionBodyPart = ( TypeBodyElement | ReturnFeatureMember )*
//                                      ( ResultExpressionMember )?
// rejects: PARSE-EXPECTED on: function Average { in scores : Rational; sum(scores) / size(scores); }
package Functions {
	function Average { in scores : Rational; sum(scores) / size(scores); }
}
