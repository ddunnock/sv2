// Rule: a Documentation ends in its REGULAR_COMMENT body; the body is not optional.
// SysML 8.2.2.4.2 — Documentation = 'doc' Identification ( 'locale' locale = STRING_VALUE )?
//   body = REGULAR_COMMENT
// `Intro` is only the Identification, and nothing follows it before the '}'.
public import A::* { doc Intro }
