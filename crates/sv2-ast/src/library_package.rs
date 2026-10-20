// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One library package: whether it is standard, its prefix metadata, its declaration
//! and its body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// A library package, in either language.
///
/// ```text
/// LibraryPackage = ( isStandard ?= 'standard' ) 'library'
///                  ( ownedRelationship += PrefixMetadataMember )*
///                  PackageDeclaration PackageBody   (SysML 8.2.2.5.1, KerML 8.2.5.13)
/// ```
///
/// read with deviation `LibraryPackage` (`follow_xtext`), which makes `'standard'`
/// optional: the clauses print the group without a `?`, and every library package in the
/// corpus writes `library package` alone. One grammar unit for both languages
/// (`.claude/state/grammar/units/LibraryPackage.json`), and this view is the same for
/// both, as `Package`'s is.
///
/// `Package` with two keywords in front, and the metadata after them rather than first.
/// The keywords are this node's own tokens; the three slots after them are children with
/// node kinds of their own and no view yet, so all three are returned raw, to be upgraded
/// in the change that ships each view (`.claude/rules/ast.md`, the delegation ripple).
///
/// Every one of the 10 library packages across the repository's 610 trees holds exactly
/// one declaration and one body, which `tests/sweep.rs` asserts as equality, together with
/// the order the production writes them in.
// node: LibraryPackage
pub struct LibraryPackage(SyntaxNode);

impl AstNode for LibraryPackage {
    const KIND: SyntaxKind = SyntaxKind::LibraryPackage;

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

impl LibraryPackage {
    /// Whether `standard` is written before `library`: the metaclass's `isStandard`,
    /// which "should only be set to true" for the recognised standard libraries (`KerML`
    /// 8.3.4.13.3).
    ///
    /// Read from this node's own tokens, never its descendants: a standard library package
    /// nested in the body is not this one. **No pinned corpus file writes `standard`**, so
    /// the case that exercises this is hand-written (`tests/positive.rs`).
    pub fn is_standard(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::KwStandard)
    }

    /// The `PrefixMetadataMember`s written between `library` and `package`, in source
    /// order.
    ///
    /// Raw, because `PrefixMetadataMember` has no view yet. Usually none: **no pinned
    /// corpus file writes prefix metadata on a library package**, so the case that
    /// exercises this is hand-written (`tests/positive.rs`). Metadata written before
    /// `library`, in the place a `Package` writes its own, is not one of these and is not
    /// this package's: it is left in an `Error` beside the package's member
    /// (`tests/malformed/library-package/prefix-metadata-before-library.sysml`).
    pub fn prefix_metadata(&self) -> impl Iterator<Item = SyntaxNode> {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::PrefixMetadataMember)
    }

    /// The `PackageDeclaration`: the `package` keyword and the package's identification.
    ///
    /// Raw, because `PackageDeclaration` has no view yet. As for `Package::declaration`,
    /// **present is not evidence that a name was written**: `package_declaration`
    /// (`crates/sv2-syntax/src/parser/namespace.rs`) builds its `Identification` whether or
    /// not a name follows `package`. Read `Parse::errors()`.
    ///
    /// Read from the direct children. Reading descendants instead cannot be observed —
    /// the mutation survives the sweep — because nothing before the declaration can hold
    /// one, which `tests/sweep.rs` asserts; so preorder reaches this declaration first.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::PackageDeclaration)
    }

    /// The `PackageBody`: a `;`, or the braced run of members.
    ///
    /// Raw, because `PackageBody` has no view yet. Built whether or not either form was
    /// written, by `package_body` (`crates/sv2-syntax/src/parser/namespace.rs`), which
    /// opens the node first and reports what is missing inside it.
    ///
    /// Read from the direct children. Reading descendants instead cannot be observed, for
    /// the same reason as for `declaration`: the body is the last child, and nothing
    /// before it can hold one, both of which `tests/sweep.rs` asserts.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::PackageBody)
    }

    /// The first direct child of `kind`.
    ///
    /// First, not only: the production states one declaration and one body, and
    /// `tests/sweep.rs` asserts exactly one of each over every tree in the repository.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }
}
