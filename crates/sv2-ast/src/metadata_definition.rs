// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A metadata definition: its `abstract`, its extension keywords, its `metadata def`
//! keywords and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;

/// A metadata definition.
///
/// ```text
/// MetadataDefinition = ( isAbstract ?= 'abstract' )? DefinitionExtensionKeyword*
///                      'metadata' 'def' Definition
/// ```
///
/// `SysML` 8.2.2.27. Built by `metadata_definition`
/// (`crates/sv2-syntax/src/parser/metadata.rs`). Like `EnumerationDefinition` it has **no
/// prefix node**: its prefix is its own and narrower than `DefinitionPrefix`, `abstract`
/// and never `variation`, so `abstract` is this node's own token and its extension
/// keywords are its own children, as many as are written. A `variation` written before it
/// is an `Error` outside the node
/// (`tests/malformed/metadata-definition/variation-prefix.sysml`).
///
/// **Both keywords are always there.** `at_metadata_definition` begins one only on
/// `metadata` followed by `def`, so `metadata A def B;` is a `MetadataUsage` and builds no
/// metadata definition at all (`def-missing.sysml` beside the case above). A missing `def`
/// is therefore a missing node, never a node missing its keyword, and the keyword
/// accessors answer `None` on no tree this parser builds.
///
/// **`SysML` only.** `KerML` has no `MetadataDefinition` and does not reserve `def`
/// (`KerML` 8.2.2.6), so the same words in a `.kerml` file are a `MetadataFeature` named
/// `def` (8.2.5.12), and this view never sees them.
///
/// `DefinitionExtensionKeyword` has a kind of its own and no view yet, so it is returned
/// raw until it does. `Definition` has a view, and is returned typed.
// node: MetadataDefinition
pub struct MetadataDefinition(SyntaxNode);

impl AstNode for MetadataDefinition {
    const KIND: SyntaxKind = SyntaxKind::MetadataDefinition;

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

impl MetadataDefinition {
    /// The `abstract` keyword, when written. It is this node's own token: the clause
    /// writes it inline, and there is no prefix node to own it.
    pub fn abstract_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwAbstract)
    }

    /// The `DefinitionExtensionKeyword`s, in source order: each a `#` and the metadata it
    /// names. Usually none.
    ///
    /// Raw `SyntaxNode`s of kind `DefinitionExtensionKeyword`, because that kind has no
    /// view yet. Read from the direct children, and that matters: the body may hold a
    /// definition with keywords of its own, which are not this one's. Any at all, their
    /// order, their place after `abstract`, and a nested definition's being left out are
    /// pinned by `tests/positive.rs`, because no corpus tree writes any of them.
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }

    /// The `metadata` keyword.
    pub fn metadata_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwMetadata)
    }

    /// The `def` keyword: the one written straight after `metadata`. A second `def` is not
    /// this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/metadata-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `Definition`: the declaration and the body. Built even when nothing is written,
    /// so present and zero width after a `def` with no name; ask it for its declaration
    /// and body, and read `Parse::errors()`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs` assertion.
    /// Taking the last is equivalent because the sweep asserts the node has exactly one
    /// `Definition`. Searching descendants is equivalent because it also asserts that no
    /// extension keyword holds one, so the node's own is the first in preorder, ahead of
    /// any written in its body.
    pub fn definition(&self) -> Option<Definition> {
        self.0.children().find_map(Definition::cast)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Taking the last token of the kind cannot be observed, because the sweep asserts the
    /// node's elements are exactly the production's, at most one of each keyword.
    ///
    /// Searching descendant tokens is observable, and caught, for `abstract` alone. For
    /// `metadata` and `def`, which every node writes, the node's own comes first in
    /// preorder: the sweep asserts the extension keywords, the only nodes before them, hold
    /// neither. But a node with no `abstract` of its own would answer one written in its
    /// body, and `tests/positive.rs` writes that, since no corpus tree does. Taking the
    /// first own token whatever its kind is caught too: it answers `metadata` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
