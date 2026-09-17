// Rule: SPECIALIZES is followed by at least one OwnedSubclassification.
// SysML 8.2.2.6.5 — SubclassificationPart = SPECIALIZES ownedRelationship += OwnedSubclassification
//   ( ',' ownedRelationship += OwnedSubclassification )*
// SysML 8.2.2.6.5 — OwnedSubclassification = superClassifier = [QualifiedName]
part def Vehicle :> ;
