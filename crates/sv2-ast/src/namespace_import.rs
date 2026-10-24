// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A namespace import: the namespace it names or the filter it carries, and whether it
//! recurses.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::qualified_name::QualifiedName;

/// One namespace import, the second alternative of `ImportDeclaration`.
///
/// ```text
/// NamespaceImport = importedNamespace = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
///                 | importedNamespace = FilterPackage
///                   { ownedRelatedElement += importedNamespace }
/// ```
///
/// `KerML` 8.2.3.4.2 and `SysML` 8.2.2.5.1, one shared unit. Its parent is an
/// `ImportDeclaration` (475 across the corpus), or in `SysML` a `NamespaceExpose` (6),
/// which wraps the same alternative without a declaration (8.2.2.26.2).
///
/// **The two alternatives hold different children.** The first holds a `QualifiedName`
/// and its own `::`, `*` and optional `::` `**` (470 in the corpus, 2 recursive). The
/// second holds one `FilterPackage` and no token of its own (11). This parser opens it
/// around an import it has already read (`filter_package`,
/// `crates/sv2-syntax/src/parser/import.rs`), so the filtered import's name and `**` lie
/// inside the filter package and are not this node's. Every filter in the corpus wraps a
/// recursive *membership* import, `vehicle::**[@Safety]`.
///
/// The name is a child with a view of its own, so it is returned typed. The filter
/// package has a kind of its own and no view yet, so it is returned raw until it has one.
// node: NamespaceImport
pub struct NamespaceImport(SyntaxNode);

impl AstNode for NamespaceImport {
    const KIND: SyntaxKind = SyntaxKind::NamespaceImport;

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

impl NamespaceImport {
    /// The `QualifiedName` of the namespace imported, in the first alternative. `None` in
    /// the second, which names nothing of its own.
    ///
    /// Read from the direct children: in the second alternative a name lies further down,
    /// inside the filter package, and belongs to the import it filters. `expose
    /// vehicle::**[@Safety and ...];` is the pinned case (`tests/positive.rs`, 11b, line
    /// 61). The first child of the kind or the last cannot be told apart, because
    /// `tests/sweep.rs` asserts the node holds one child.
    pub fn name(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// The `FilterPackage` imported, in the second alternative. `None` in the first.
    ///
    /// A raw `SyntaxNode` of kind `FilterPackage`, because that kind has no view yet. It
    /// holds the filtered import's declaration (in `SysML` wrapped in a
    /// `FilterPackageImport`) and then one `FilterPackageMember` per `[...]`.
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the node holds one child, and that a `QualifiedName` holds tokens only, so
    /// the first `FilterPackage` beneath the node is always its child.
    pub fn filter_package(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::FilterPackage)
    }

    /// Whether `::**` is written after the `::*`: `isRecursive`, which imports the
    /// namespace's memberships and, recursively, those of the namespaces among them.
    ///
    /// Read from this node's own tokens, as a `**`. A second `::` alone is not the suffix,
    /// and `public import A::*::;` leaves one here with no `**` after it
    /// (`tests/malformed/namespace-import/recursive-suffix-unfinished.sysml`).
    ///
    /// Always `false` in the second alternative, which assigns no `isRecursive`. Its
    /// filtered import may write a `**`, as every filter in the corpus does, but that
    /// `**` is the inner import's. Ask that import for it.
    pub fn is_recursive(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::StarStar)
    }
}
