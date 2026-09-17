// Rule: every ',' in a SubclassificationPart is followed by an OwnedSubclassification.
// SysML 8.2.2.6.5 — SubclassificationPart = SPECIALIZES ownedRelationship += OwnedSubclassification
//   ( ',' ownedRelationship += OwnedSubclassification )*
part def Car :> Vehicle, ;
