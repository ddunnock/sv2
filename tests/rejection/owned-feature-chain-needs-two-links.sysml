// NOT A REJECTION OF `a` — this file rejects on `.` alone with nothing before it.
// SysML 8.2.2.6.5 — OwnedFeatureChain = OwnedFeatureChaining
//                                       ( '.' OwnedFeatureChaining )+
// The `+` means a chain is at least TWO links, so a bare `a` after `:>>` is the
// QualifiedName alternative and never a chain of one. That case parses and is asserted
// in crates/sv2-syntax/tests/parser.rs; what cannot happen is a chain with no first
// link, because the first link is what the alternative is chosen after.
package P {
	part p :>> .b;
}
