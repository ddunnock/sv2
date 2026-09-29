// REJECTED BY RULE. A function body's result expression is its last element and is
// written without a final semicolon: it is a ResultExpressionMember, an expression, and
// not a member ending in a body.
// KerML 8.2.5.7.1 — FunctionBodyPart       = ( TypeBodyElement | ReturnFeatureMember )*
//                                            ( ResultExpressionMember )?
//                   ResultExpressionMember = MemberPrefix OwnedExpression
// rejects: PARSE-EXPECTED on: inv { fuelLevel >= 0; }
class FuelTank {
	feature fuelLevel : Real;
	inv { fuelLevel >= 0; }
}
