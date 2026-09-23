// REJECTED BY RULE. A filter condition is one OwnedExpression between brackets, so an
// empty `[]` filters by nothing. The corpus always writes a condition:
// `public import vehicle::**[@Safety];` (training/40. Filtering/Filtering
// Example-2.sysml:28).
// SysML 8.2.2.5.1 - FilterPackage       = FilterPackageImport FilterPackageMember+
//                   FilterPackageMember = '[' ownedRelatedElement += OwnedExpression ']'
// rejects: PARSE-EXPECTED on: public import vehicle::**[];
package 'Safety Features' {
	public import vehicle::**[];
}
