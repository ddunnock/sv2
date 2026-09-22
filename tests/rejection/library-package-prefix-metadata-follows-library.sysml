// REJECTED BY RULE. A library package's prefix metadata stands between `library` and
// `package`, not before `library`: LibraryPackage writes its PrefixMetadataMember* after the
// keyword, where Package, which has none, writes it first.
// SysML 8.2.2.5.1 — LibraryPackage = ( isStandard ?= 'standard' ) 'library'
//                     ( ownedRelationship += PrefixMetadataMember )*
//                     PackageDeclaration PackageBody
// rejects: PARSE-UNEXPECTED on: #Approved library package FMEALibrary;
#Approved library package FMEALibrary;
