// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A membership import: the name it imports, and whether it recurses.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::qualified_name::QualifiedName;

/// One membership import, the first alternative of `ImportDeclaration`.
///
/// ```text
/// MembershipImport = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
/// ```
///
/// `KerML` 8.2.3.4.2 and `SysML` 8.2.2.5.1, one shared unit. Its parent is an
/// `ImportDeclaration` (269 across the corpus), or in `SysML` a `MembershipExpose` (4),
/// which wraps the same alternative without a declaration (8.2.2.26.2). It is told from
/// a `NamespaceImport` by the absence of `::*`, never by the absence of stars: `Q::**` is
/// this alternative, recursive (`ImportDeclaration::imported`).
///
/// The name is a child with a view of its own, so it is returned typed. The suffix's `::`
/// and `**` are this node's own tokens; the `::` inside a multi-segment name is the
/// name's, not this node's.
// node: MembershipImport
pub struct MembershipImport(SyntaxNode);

impl AstNode for MembershipImport {
    const KIND: SyntaxKind = SyntaxKind::MembershipImport;

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

impl MembershipImport {
    /// The `QualifiedName` of the membership imported.
    ///
    /// **Present is not evidence that anything was named.** `qualified_name`
    /// (`crates/sv2-syntax/src/parser/import.rs`) opens its node before it seeks a name, so
    /// `public import ::**;` yields a name with no segments
    /// (`tests/malformed/membership-import/recursive-names-nothing.sysml`). Ask the name for
    /// its segments, and read `Parse::errors()`.
    ///
    /// The first child of the kind, read from the direct children; the last, or the first
    /// in preorder, cannot be told apart, because `tests/sweep.rs` asserts the name is the
    /// node's only child.
    pub fn name(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Whether `::**` is written after the name: `isRecursive`, which imports the
    /// membership and, recursively, the memberships of its namespace.
    ///
    /// Read from this node's own tokens, as a `**`: a `::` alone is not the suffix, and
    /// `public import A::;` leaves one here with no `**` after it
    /// (`tests/malformed/import-declaration/trailing-separator.sysml`). Independent of the
    /// name: a `**` written after a missing name is still reported.
    ///
    /// The `**` is a token, never a character: `A::'**'` quotes it into a name segment and
    /// is not recursive (`tests/positive.rs`). Reading descendants rather than own tokens
    /// cannot be observed, because `tests/sweep.rs` asserts no `**` inside the name.
    pub fn is_recursive(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::StarStar)
    }
}
