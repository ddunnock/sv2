// REJECTED BY DEVIATION. `new` is ConstructorExpression's keyword (KerML 8.2.5.8.3), so it
// names no feature. KerML's printed RESERVED_KEYWORD (8.2.2.6) omits it, and read literally
// that list would admit this declaration; the register reserves it anyway, by deviation
// ConstructorExpression (follow_xtext): SysML's list (8.2.2.1.2) reserves it, the pilot
// lexer reserves it in both languages, and the corpus never names anything `new`.
// KerML 8.2.4.3.1  — Feature = FeaturePrefix ( 'feature' | ... ) FeatureDeclaration? ...
// rejects: PARSE-EXPECTED on: feature new;
feature new;
