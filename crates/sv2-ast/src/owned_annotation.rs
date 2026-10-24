// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An owned annotation: the annotating element a relationship owns.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// One annotation owned by a relationship, an item of a braced `RelationshipBody`.
///
/// ```text
/// OwnedAnnotation : Annotation = ownedRelatedElement += AnnotatingElement
/// AnnotatingElement = Comment | Documentation | TextualRepresentation | MetadataFeature
/// ```
///
/// `SysML` 8.2.2.4.1 and `KerML` 8.2.3.3.1. Its parent is always a `RelationshipBody`
/// (`SysML` 8.2.2.2, `KerML` 8.2.3.1). The corpus writes only 8, all `SysML`: 5 under an
/// `Import`, 3 under an `AliasMember`, each owning a `Comment` (3) or a `Documentation`
/// (5). A comment or documentation owned as a *member* is not one of these. That is
/// the far more common case, and those elements sit in a namespace body with no
/// `OwnedAnnotation` around them.
///
/// The node holds the element and no token of its own.
// node: OwnedAnnotation
pub struct OwnedAnnotation(SyntaxNode);

impl AstNode for OwnedAnnotation {
    const KIND: SyntaxKind = SyntaxKind::OwnedAnnotation;

    fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == Self::KIND {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl OwnedAnnotation {
    /// The annotating element this annotation owns: its `ownedRelatedElement`.
    ///
    /// Untyped because `AnnotatingElement` is an alternation that builds **no node of its
    /// own**, the second of the three cases in `.claude/rules/ast.md`, as
    /// [`NonFeatureMember::element`](crate::NonFeatureMember::element) is. What comes back
    /// is the concrete element, so match on `kind()`: a `Comment`, a `Documentation`, a
    /// `TextualRepresentation`, or the fourth alternative. That is a `MetadataUsage` in
    /// `SysML` (deviation `AnnotatingElement`, `follow_xtext`) and a `MetadataFeature` in
    /// `KerML`.
    ///
    /// **Present is not evidence that the element is whole.** `owned_annotation`
    /// (`crates/sv2-syntax/src/parser/annotation.rs`) is reached only where an annotating
    /// element starts, so this is never `None` on a tree the parser built. But the element
    /// may be cut short: `alias A for B { doc }` yields a `Documentation` with no body
    /// (`tests/malformed/owned-annotation/documentation-body-missing.sysml`). Read
    /// `Parse::errors()`.
    ///
    /// The first child; the node has only one, which `tests/sweep.rs` asserts.
    pub fn element(&self) -> Option<SyntaxNode> {
        self.0.first_child()
    }
}
