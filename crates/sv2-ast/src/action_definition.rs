// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An action definition: its prefix, its `action def` keywords, its declaration and its
//! body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;

/// An action definition.
///
/// ```text
/// ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
///                    DefinitionDeclaration ActionBody
/// ```
///
/// `SysML` 8.2.2.17.1. Built by `action_definition`
/// (`crates/sv2-syntax/src/parser/action.rs`) on
/// [`RequirementDefinition`](crate::RequirementDefinition)'s spine: it takes the
/// declaration and the body directly, so there is **no `Definition` node** here, and the
/// two are this view's own children. 121 in the corpus, all under `PackageMember`, four
/// of them prefixed (`individual` three times, `variation` once).
///
/// **Both keywords are always there.** `at_action_definition` begins an action
/// definition only on `action` followed by `def`, so `action A def B;` is an `ActionUsage`
/// and builds no action definition at all
/// (`tests/malformed/action-definition/def-missing.sysml`). A missing `def` is
/// therefore a missing node, never a node missing its keyword, and the keyword accessors
/// answer `None` on no tree this parser builds.
///
/// The prefix has a view and is returned typed. `DefinitionDeclaration` and
/// `ActionBody` have kinds of their own and no views yet, so they are returned raw
/// until they do. The keywords are this node's own tokens.
// node: ActionDefinition
pub struct ActionDefinition(SyntaxNode);

impl AstNode for ActionDefinition {
    const KIND: SyntaxKind = SyntaxKind::ActionDefinition;

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

impl ActionDefinition {
    /// The `OccurrenceDefinitionPrefix`: `abstract` or `variation`, `individual`, and the
    /// user-defined keywords. Built even when nothing is written, so usually present and
    /// zero width (`OccurrenceDefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<OccurrenceDefinitionPrefix> {
        self.0.children().find_map(OccurrenceDefinitionPrefix::cast)
    }

    /// The `action` keyword.
    pub fn action_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwAction)
    }

    /// The `def` keyword: the one written straight after `action`. A second `def` is
    /// not this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/action-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `DefinitionDeclaration`: the identification and the subclassification part.
    /// Built even when nothing is written, so present and zero width after a `def` with
    /// no name.
    ///
    /// A raw `SyntaxNode` of kind `DefinitionDeclaration`, because that kind has no view
    /// yet.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DefinitionDeclaration)
    }

    /// The `ActionBody`: `;`, or `{` and its items and `}`. Zero width when neither
    /// `;` nor `{` follows the declaration, which the parser reports.
    ///
    /// A raw `SyntaxNode` of kind `ActionBody`, because that kind has no view yet.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::ActionBody)
    }

    /// The first direct child of `kind`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs` assertion.
    /// Taking the last is equivalent because the sweep asserts the node's elements are
    /// exactly `OccurrenceDefinitionPrefix 'action' 'def' DefinitionDeclaration
    /// ActionBody`, one of each. Searching descendants is equivalent because it also
    /// asserts the prefix holds neither kind and the declaration holds no body, so each is
    /// the first of its kind in preorder; the body, which may nest another action
    /// definition, comes after both.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last token of the kind is equivalent because
    /// the sweep asserts the node's elements are exactly the production's, one of each.
    /// Searching descendant tokens is equivalent because it also asserts the prefix, the
    /// only element before the keywords, holds neither; an action definition nested
    /// in the body comes after them in preorder. Taking the first own token whatever its
    /// kind is not equivalent, and is caught: it answers `action` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
