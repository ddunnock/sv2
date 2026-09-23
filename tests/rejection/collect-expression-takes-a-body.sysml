// REJECTED BY RULE. A collect expression's only argument is a body: CollectExpression
// writes BodyArgumentMember after '.', and no production writes '.' before an argument
// list. The corpus writes the body form: `x.{in xx; xx + 1}`
// (vendor/corpus/kerml/src/examples/Simple Tests/Expressions.kerml:16).
// KerML 8.2.5.8.2 - CollectExpression =
//                     ownedRelationship += PrimaryArgumentMember '.'
//                     ownedRelationship += BodyArgumentMember
// rejects: PARSE-EXPECTED on: attribute x = things.(a);
attribute x = things.(a);
