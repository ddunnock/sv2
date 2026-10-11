// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The trait every typed accessor implements.

use sv2_syntax::{SyntaxKind, SyntaxNode};

/// A typed view of one CST node.
///
/// Implementors own no data: the type wraps one [`SyntaxNode`] and nothing else, so the
/// tree stays the single source of truth and the text stays authoritative (ADR-0001).
pub trait AstNode: Sized {
    /// The kind this view is a view of.
    const KIND: SyntaxKind;

    /// A view of `node`, or `None` when `node` is some other kind.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// The node this view borrows.
    fn syntax(&self) -> &SyntaxNode;
}
