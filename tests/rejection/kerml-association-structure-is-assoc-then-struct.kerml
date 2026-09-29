// REJECTED BY RULE. An AssociationStructure is declared with the two keywords `assoc`
// then `struct`. In the other order, `struct` opens a Structure, and `assoc` cannot be
// its name: it is reserved.
// KerML 8.2.5.4 — AssociationStructure = TypePrefix 'assoc' 'struct'
//                                        ClassifierDeclaration TypeBody
// rejects: PARSE-EXPECTED on: struct assoc ExtendedAssetOwnership;
package Associations {
	struct assoc ExtendedAssetOwnership;
}
