// REJECTED BY ABSENCE. ConstructorExpression is BaseExpression's OTHER alternative that
// ends in an ArgumentList, and implementing ArgumentList does not implement it: the
// `new` keyword is not read.
// KerML 8.2.5.8.3 — BaseExpression        = NullExpression | LiteralExpression
//                                         | FeatureReferenceExpression
//                                         | MetadataAccessExpression | InvocationExpression
//                                         | ConstructorExpression | BodyExpression
// This case exists so that ArgumentList landing is not read as `new` landing with it.
// The pinned corpus writes both forms in one file — vendor/corpus/sysml/src/examples/
// Simple Tests/ParameterTest.sysml has `F(a, 2)` and `new A(y=a, x="")` — so that file
// still does not parse, and this names the reason.
calc def C {
	new A(y = a)
}
