// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One package: its prefix metadata, its declaration and its body.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// A package, in either language.
///
/// ```text
/// Package = ( ownedRelationship += PrefixMetadataMember )*
///           PackageDeclaration PackageBody          (SysML 8.2.2.5.1, KerML 8.2.5.13)
/// ```
///
/// One grammar unit for both languages (`.claude/state/grammar/units/Package.json`):
/// the two clauses state it identically, and each language resolves the productions it
/// names to its own — `SysML`'s `PackageBody` reads `PackageBodyElement`, `KerML`'s
/// `NamespaceBodyElement`. A `SysML` package is owned through a `PackageMember`, a `KerML`
/// one through a `NonFeatureMember`, and this view is the same for both.
///
/// All three slots are children with node kinds of their own, and none has a view yet, so
/// all three are returned raw. When a view for one ships, the accessor here is upgraded to
/// it in the same change (`.claude/rules/ast.md`, the delegation ripple).
///
/// The declaration and the body are mandatory, and every one of the 666 packages across
/// the repository's 610 trees holds exactly one of each — which `tests/sweep.rs` asserts
/// as equality, together with the order the production writes them in.
// node: Package
pub struct Package(SyntaxNode);

impl AstNode for Package {
    const KIND: SyntaxKind = SyntaxKind::Package;

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

impl Package {
    /// The `PrefixMetadataMember`s written before `package`, in source order.
    ///
    /// Raw, because `PrefixMetadataMember` has no view yet. Usually none: **no pinned
    /// corpus file writes prefix metadata on a package**, so the case that exercises this
    /// is hand-written (`tests/positive.rs`). A `#` that names nothing is not one of these
    /// and is not this package's: it is left in an `Error` beside the package's member
    /// (`tests/malformed/package/prefix-metadata-names-nothing.sysml`).
    pub fn prefix_metadata(&self) -> impl Iterator<Item = SyntaxNode> {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::PrefixMetadataMember)
    }

    /// The `PackageDeclaration`: the `package` keyword and the package's identification.
    ///
    /// Raw, because `PackageDeclaration` has no view yet. **Present is not evidence that a
    /// name was written**: `package_declaration` (`crates/sv2-syntax/src/parser/`
    /// `namespace.rs`) is entered only once `package` has been seen and builds its
    /// `Identification` whether or not a name follows, so `package` alone yields a
    /// declaration with an empty identification. Read `Parse::errors()`.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::PackageDeclaration)
    }

    /// The `PackageBody`: a `;`, or the braced run of members.
    ///
    /// Raw, because `PackageBody` has no view yet. Built whether or not either form was
    /// written: `package_body` (`crates/sv2-syntax/src/parser/namespace.rs`) opens the
    /// node first and reports a missing `;` or `{` inside it, so a declaration with
    /// nothing after it has a zero-width body rather than none.
    ///
    /// Read from the direct children. Reading descendants instead cannot be observed —
    /// the mutation survives the sweep — because the body is the last child and nothing
    /// before it can hold one, both of which `tests/sweep.rs` asserts; so preorder reaches
    /// this body before any nested inside it.
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
