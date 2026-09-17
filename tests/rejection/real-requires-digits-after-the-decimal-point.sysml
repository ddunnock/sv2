// REJECTED BY RULE. The part after the point is required, so `1.` is not a
// RealValue. It is a DECIMAL_VALUE followed by a `.` that no production accepts
// here, and the `.` is reported. `1.5` and `.5` are both RealValues.
// KerML 8.2.5.8.4 — RealValue = DECIMAL_VALUE? '.' ( DECIMAL_VALUE
//                               | EXPONENTIAL_VALUE ) | EXPONENTIAL_VALUE
attribute x = 1.;
