// Clause: SysML 8.2.2.4.1 (OwnedAnnotation, AnnotatingElement), 8.2.2.4.2 (Documentation)
// and 8.2.2.2 (RelationshipBody) --
//
//   OwnedAnnotation : Annotation = ownedRelatedElement += AnnotatingElement
//   AnnotatingElement = Comment | Documentation | TextualRepresentation | MetadataFeature
//   Documentation = 'doc' Identification ( 'locale' locale = STRING_VALUE )?
//                   body = REGULAR_COMMENT
//   RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
//
// The body of a Documentation is not optional. This file writes `doc` inside an alias's
// braced body and closes the braces before any `/* ... */`: `alias A for B { doc }`. Every
// one of the 8 OwnedAnnotations in the corpus owns a whole element, so the corpus never
// shows the annotation begun and its element cut short. That is the shape in which
// "an annotation was written" and "the element it owns is well-formed" come apart.
//
// Recovery (NOT a clause, and NOT an ADR): `owned_annotations`
// (crates/sv2-syntax/src/parser/annotation.rs) sees `doc` start an annotating element and
// calls `owned_annotation`, which opens the node and calls `annotating_element`.
// `documentation` (same file) takes `doc` and an empty Identification, and reports the `}`
// it finds where the body should be without consuming it. So the OwnedAnnotation closes
// around a Documentation with no body, and `owned_annotations`' own `expect` takes the `}`:
//
//   AliasMember "alias A for B {\n\t\tdoc\n\t}"
//     RelationshipBody "{ ... }"
//       OwnedAnnotation "doc..."      <- element(): the Documentation, present
//         Documentation "doc..."      <- KwDoc, an empty Identification, NO RegularComment
//       '}'                           <- still the body's terminator
//
// What that pins:
//
//  1. The parse still yields a tree, holding one OwnedAnnotation.
//  2. element() reports what was written, a Documentation, rather than hiding a damaged
//     element as absent. The damage is the element's own slot, its body, and something
//     reported it: one diagnostic, at the `}` where the body should be. element() itself
//     is never None on a node the parser built, because `owned_annotation` is reached only
//     where an annotating element starts.
//  3. The neighbours still answer. The RelationshipBody's elements() is this one
//     annotation and its terminator is still the `}`, and the AliasMember still names `A`
//     and targets `B`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.4.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	alias A for B {
		doc
	}
}
