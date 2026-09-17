// Rule: 'locale' is followed by a STRING_VALUE.
// SysML 8.2.2.4.2 — Documentation = 'doc' Identification ( 'locale' locale = STRING_VALUE )?
//   body = REGULAR_COMMENT
// The comment here is the body, so the locale has no string.
public import A::* { doc locale /* body */ }
