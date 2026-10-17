// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The root of one parsed file.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The whole file: zero or more `PackageBodyElement` in `SysML`, zero or more
/// `NamespaceBodyElement` in `KerML` — the one production the two grammars state
/// differently at the start symbol (ADR-0014; `crates/sv2-syntax/src/parser/body.rs`).
/// Every tree has exactly one.
// node: RootNamespace
pub struct RootNamespace(SyntaxNode);

impl AstNode for RootNamespace {
    const KIND: SyntaxKind = SyntaxKind::RootNamespace;

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

impl RootNamespace {
    /// This namespace's top-level members, in source order: every direct child of the
    /// root, with nothing filtered out.
    ///
    /// A production swallowed into an earlier, unclosed sibling's body is not one of
    /// these: this walks the tree the parser actually built, never the member count
    /// the source text seems to promise (`tests/malformed/root-namespace/unclosed-body.sysml`).
    ///
    /// Recovery nodes are members. A `SyntaxKind::Error` is not an alternative of
    /// `PackageBodyElement` (`SysML` 8.2.2.5.1) or `NamespaceBodyElement` (`KerML`
    /// 8.2.3.4.1), yet one appears here whenever the parser could not make sense of
    /// top-level text — 64 times across the 299 rejection files. They are yielded
    /// deliberately, not by oversight: ADR-0002 admits into the IR everything that
    /// parses, carrying its diagnostics, and a caller that cannot see the damage cannot
    /// report it. Match on `kind()` to tell a member from a wound; read
    /// `Parse::errors()` for what went wrong.
    pub fn members(&self) -> impl Iterator<Item = SyntaxNode> {
        self.0.children()
    }
}
