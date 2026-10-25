// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A definition's declaration and body, the part every simple definition shares.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The declaration and body of a definition.
///
/// ```text
/// Definition = DefinitionDeclaration DefinitionBody
/// ```
///
/// `SysML` 8.2.2.6.1. It follows the prefix and keywords of the eight definitions
/// `simple_definition` builds (`crates/sv2-syntax/src/parser/definition.rs`), and of
/// `ExtendedDefinition`, `IndividualDefinition`, `PortDefinition` and
/// `MetadataDefinition`: 1,064 in the corpus, under 12 parent kinds, `PartDefinition` (544)
/// the most. It is **not** in every definition. The action, state, calculation,
/// constraint, interface and view definitions read a `DefinitionDeclaration` and a body of
/// their own, with no `Definition` around them.
///
/// The node holds exactly its two children, in that order, and no token of its own but
/// trivia. Both
/// have kinds of their own and no view yet, so they are returned raw until they do.
// node: Definition
pub struct Definition(SyntaxNode);

impl AstNode for Definition {
    const KIND: SyntaxKind = SyntaxKind::Definition;

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

impl Definition {
    /// The `DefinitionDeclaration`: the identification and the specializations.
    ///
    /// A raw `SyntaxNode` of kind `DefinitionDeclaration`, because that kind has no view
    /// yet.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DefinitionDeclaration)
    }

    /// The `DefinitionBody`: `;`, or `{` the definition's members `}`.
    ///
    /// A raw `SyntaxNode` of kind `DefinitionBody`, because that kind has no view yet.
    ///
    /// **Present is not evidence that a body was written.** `definition_body`
    /// (`crates/sv2-syntax/src/parser/definition.rs`) opens its node before it looks for
    /// `;` or `{`, so `part def A` followed by another member leaves a zero-width body
    /// (`tests/malformed/definition/body-missing-before-next-member.sysml`). Ask the node
    /// for its tokens, and read `Parse::errors()`.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DefinitionBody)
    }

    /// The first direct child of `kind`.
    ///
    /// Four mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last child of the kind, the first child or
    /// the last child are equivalent, because the sweep asserts the node's children are
    /// exactly `DefinitionDeclaration DefinitionBody`. Searching descendants is equivalent
    /// because it also asserts that the declaration holds neither kind. A body's nested
    /// definitions come after the node's own declaration and body in preorder, and
    /// `tests/positive.rs` pins them with `CartSample.sysml`.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }
}
