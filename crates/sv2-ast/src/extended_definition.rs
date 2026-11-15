// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An extended definition: a definition declared by its user-defined keywords alone, its
//! basic prefix, its `def` and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;

/// An extended definition.
///
/// ```text
/// ExtendedDefinition : Definition =
///     BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition
/// ```
///
/// `SysML` 8.2.2.27: "A user-defined keyword for semantic metadata may also be used to
/// declare a definition or usage without using any language-defined keyword" (7.27.4).
/// Built by `extended_definition` (`crates/sv2-syntax/src/parser/definition.rs`).
///
/// **Its prefix has no node.** Like `IndividualDefinition`, the `BasicDefinitionPrefix` and
/// the extension keywords are this node's own children, written inline. There is no kind
/// keyword: the `#` keywords stand in its place, so there is always at least one.
///
/// **The keywords and `def` are always there.** `at_extended_definition` begins one only on
/// at least one `#` keyword followed by `def`. `def A;` begins none and is an `Error`
/// (`tests/malformed/extended-definition/extension-keyword-missing.sysml`), as is `#X
/// abstract def A;` (`basic-prefix-after-keyword.sysml` beside it), so a missing or
/// misplaced word is a missing node, `extension_keywords()` is never empty, and
/// `def_keyword()` answers `None` on no tree this parser builds.
///
/// `Definition` has a view and is returned typed. The basic prefix and the extension
/// keywords have kinds of their own and no views yet, so they are returned raw until they
/// do.
// node: ExtendedDefinition
pub struct ExtendedDefinition(SyntaxNode);

impl AstNode for ExtendedDefinition {
    const KIND: SyntaxKind = SyntaxKind::ExtendedDefinition;

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

impl ExtendedDefinition {
    /// The `BasicDefinitionPrefix`, `abstract` or `variation`, when one is written.
    ///
    /// A raw `SyntaxNode` of kind `BasicDefinitionPrefix`, because that kind has no view
    /// yet. Which keyword it is belongs to that node: ask it for its token.
    ///
    /// Read from the direct children. Taking the last cannot be observed, because the
    /// sweep asserts the node has at most one. Searching descendants is NOT equivalent: a
    /// node with no prefix of its own would answer one written in its body, and
    /// `tests/positive.rs` nests one that would, since no corpus tree does.
    pub fn basic_prefix(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::BasicDefinitionPrefix)
    }

    /// Each `DefinitionExtensionKeyword`, a `#X` user-defined keyword, in source order: at
    /// least one.
    ///
    /// Raw nodes of kind `DefinitionExtensionKeyword`, because that kind has no view yet.
    /// Each holds the `PrefixMetadataMember` that names the metadata.
    ///
    /// Read from the direct children. Reading descendants would also yield the keywords of
    /// a definition nested in the body, and `tests/positive.rs` writes one that would,
    /// since no corpus tree does.
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }

    /// The `def` keyword: the one written after the `#` keywords. A second `def` is not
    /// this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/extended-definition/def-written-twice.sysml`).
    ///
    /// Three mutations of this cannot be observed, each under a `tests/sweep.rs`
    /// assertion. Taking the last `def`, because the node has one. Taking the first own
    /// token that is not trivia, whatever its kind, because the sweep asserts the
    /// node's elements are exactly the production's, and `def` is the only one that is a
    /// token. And searching descendant tokens, because nothing beneath a prefix slot is
    /// `def`, and the slots are all that precede it in preorder. Taking the first own
    /// token of any kind at all is caught: it answers the whitespace after a `#` keyword.
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::KwDef)
    }

    /// The `Definition`: the declaration and the body. Built even when nothing is written,
    /// so present and zero width after a `def` with no name; ask it for its declaration
    /// and body, and read `Parse::errors()`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs` assertion.
    /// Taking the last is equivalent because the node has one. Reading descendants is
    /// equivalent because the sweep asserts no prefix slot holds a `Definition`, so the
    /// node's own is the first in preorder, ahead of any written in its body.
    pub fn definition(&self) -> Option<Definition> {
        self.0.children().find_map(Definition::cast)
    }
}
