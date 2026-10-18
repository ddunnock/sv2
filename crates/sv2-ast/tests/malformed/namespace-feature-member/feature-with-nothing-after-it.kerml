// Clause: KerML 8.2.3.4.1 --
//
//   NamespaceFeatureMember : OwningMembership =
//       MemberPrefix ownedRelatedElement += FeatureElement
//
// KerML only. ADR-0015 re-scoped this unit to the kerml grammar alone because no SysML
// grammar reaches it, and the corpus agrees: all 133 NamespaceFeatureMember nodes in this
// repository's 610 trees come from .kerml files. A SysML namespace owns its members
// through PackageMember instead.
//
// Both slots are mandatory, and every one of the 133 holds exactly one of each -- so this
// kind's own SHAPE cannot be wrong, and there is no reachable absence either. What this
// fixture pins is why the second of those is true, because it is a stronger statement
// than it looks and a caller can misread it.
//
// Recovery (NOT a clause, and NOT an ADR): `feature_element`
// (crates/sv2-syntax/src/parser/feature.rs) dispatches on the reserved word that opens
// each of FeatureElement's alternatives and ENDS IN A TOTAL FALLBACK -- its final `else`
// calls `self.feature()`, because a KerML Feature's keyword is optional and so anything
// that got this far is read as one. There is therefore no input for which the element slot
// is empty, and none for which it holds a SyntaxKind::Error:
//
//   NamespaceFeatureMember "private feature\n"
//     MemberPrefix "private"      <- reports `private`, correctly
//       VisibilityIndicator "private"
//     Feature "feature\n"         <- a Feature node, with no name and no terminator
//
// What that pins:
//
//  1. element() ALWAYS answers. `is_some()` is therefore not evidence that a usable
//     feature was written -- here the Feature has no name, no typing and no `;`, and the
//     accessor still reports it. The same trap as Import::declaration, reached a different
//     way: there the node is opened before the name is sought, here the dispatch cannot
//     fail to pick an alternative.
//  2. The damage is confined to the element. The prefix beside it still reports `private`,
//     so a caller must not read a diagnostic anywhere in the member as reason to distrust
//     every slot in it.
//  3. One diagnostic, and it is about the classifier declaration rather than about the
//     membership -- `expected ';' or '{' after a classifier declaration`. The member
//     itself is not reported at all, which is the honest outcome: nothing is wrong with
//     the membership, only with what it owns.
//
// ADR-0004 supports only the shape -- every token reaches the tree, so this round-trips.
// Do not cite 8.2.3.4.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	private feature
}
