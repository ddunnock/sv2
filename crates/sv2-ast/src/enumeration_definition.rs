// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An enumeration definition: its extension keywords, its `enum def` keywords, its
//! declaration and its body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;

/// An enumeration definition.
///
/// ```text
/// EnumerationDefinition = DefinitionExtensionKeyword* 'enum' 'def'
///                         DefinitionDeclaration EnumerationBody
/// ```
///
/// `SysML` 8.2.2.8. Built by `enumeration_definition`
/// (`crates/sv2-syntax/src/parser/definition.rs`). Unlike its siblings it has **no prefix
/// node**: the grammar gives it no `abstract`, `variation` or `individual`, and its
/// extension keywords are this node's own children, as many as are written. There is no
/// `Definition` node either; the declaration and the body are this view's own children.
///
/// **Both keywords are always there.** `at_enumeration_definition` begins one only on
/// `enum` followed by `def`, so `enum A def B;` is an `EnumerationUsage` and builds no
/// enumeration definition at all
/// (`tests/malformed/enumeration-definition/def-missing.sysml`). A missing `def` is
/// therefore a missing node, never a node missing its keyword, and the keyword accessors
/// answer `None` on no tree this parser builds.
///
/// **Never nested.** The body's items are annotations and enumerated values only
/// (`EnumerationBody`, 8.2.2.8; 7.8.2), and `enum def F;` written inside one is an `Error`,
/// not a definition (`definition-in-body.sysml` beside the case above).
///
/// `DefinitionExtensionKeyword`, `DefinitionDeclaration` and `EnumerationBody` have kinds
/// of their own and no views yet, so they are returned raw until they do. The keywords are
/// this node's own tokens.
// node: EnumerationDefinition
pub struct EnumerationDefinition(SyntaxNode);

impl AstNode for EnumerationDefinition {
    const KIND: SyntaxKind = SyntaxKind::EnumerationDefinition;

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

impl EnumerationDefinition {
    /// The `DefinitionExtensionKeyword`s, in source order: each a `#` and the metadata it
    /// names. Usually none.
    ///
    /// Raw `SyntaxNode`s of kind `DefinitionExtensionKeyword`, because that kind has no
    /// view yet. Read from the direct children. Reading descendants cannot be observed: the
    /// sweep asserts that no extension keyword holds another. The order and a count above
    /// one are pinned by `tests/positive.rs`, because no corpus file writes two.
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }

    /// The `enum` keyword.
    pub fn enum_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwEnum)
    }

    /// The `def` keyword: the one written straight after `enum`. A second `def` is not this
    /// node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/enumeration-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `DefinitionDeclaration`: the identification and the subclassification part.
    /// Built even when nothing is written, so present and zero width after a `def` with no
    /// name.
    ///
    /// A raw `SyntaxNode` of kind `DefinitionDeclaration`, because that kind has no view
    /// yet.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DefinitionDeclaration)
    }

    /// The `EnumerationBody`: `;`, or `{` and its enumerated values and annotations and
    /// `}`. Zero width when neither `;` nor `{` follows the declaration, which the parser
    /// reports.
    ///
    /// A raw `SyntaxNode` of kind `EnumerationBody`, because that kind has no view yet.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::EnumerationBody)
    }

    /// The first direct child of `kind`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs` assertion.
    /// Taking the last is equivalent because the sweep asserts the node's elements are
    /// exactly `DefinitionExtensionKeyword* 'enum' 'def' DefinitionDeclaration
    /// EnumerationBody`, one of each after the extension keywords. Searching descendants is
    /// equivalent because it also asserts that no extension keyword holds either kind and
    /// the declaration holds no body, so each is the first of its kind in preorder.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last token of the kind is equivalent because
    /// the sweep asserts the node's elements are exactly the production's, one keyword of
    /// each. Searching descendant tokens is equivalent because it also asserts that the
    /// extension keywords, the only elements before the keywords, hold neither; the body,
    /// whose enumerated values may write `enum`, comes after them in preorder. Taking the
    /// first own token whatever its kind is not equivalent, and is caught: it answers
    /// `enum` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
