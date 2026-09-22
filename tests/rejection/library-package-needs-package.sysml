// REJECTED BY RULE. A library package is declared with both keywords: `library` does not
// replace `package`, it qualifies it, and PackageDeclaration opens on `package`.
// SysML 8.2.2.5.1 — LibraryPackage = ( isStandard ?= 'standard' ) 'library'
//                     ( ownedRelationship += PrefixMetadataMember )*
//                     PackageDeclaration PackageBody
//                   PackageDeclaration = 'package' Identification
// rejects: PARSE-UNEXPECTED on: library FMEALibrary {
library FMEALibrary {
	part def Situation;
}
