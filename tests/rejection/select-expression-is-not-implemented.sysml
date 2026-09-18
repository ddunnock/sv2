// REJECTED BY ABSENCE, not by rule.
// KerML 8.2.5.8.2 — SelectExpression = ownedRelationship += PrimaryArgumentMember '.?'
//                                      ownedRelationship += BodyArgumentMember
// The third dot-led postfix, and the one that needs no lookahead: '.?' lexes as a
// single DotQuestion token, so a feature chain never sees it. Here so that stays true —
// a lexer change that split the token would make this file fail rather than make
// `x.?{ ... }` parse as a chain over a body.
attribute x = things.?{ a > 1 };
