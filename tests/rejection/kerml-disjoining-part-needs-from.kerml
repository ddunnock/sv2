// REJECTED BY RULE. A DisjoiningPart opens on two keywords, `disjoint` and `from`, and
// the target follows the second: `disjoint` alone does not begin the part.
// KerML 8.2.4.1.1 — DisjoiningPart = 'disjoint' 'from' OwnedDisjoining
//                                    ( ',' OwnedDisjoining )*
// rejects: PARSE-EXPECTED on: classifier YourBike specializes Bicycle disjoint MyBike;
classifier YourBike specializes Bicycle disjoint MyBike;
