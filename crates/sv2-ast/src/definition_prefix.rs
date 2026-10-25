// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The prefix of a definition that is not an occurrence: abstract or variation, and the
//! user-defined keywords.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The prefix an attribute or port definition opens with.
///
/// ```text
/// DefinitionPrefix : Definition = BasicDefinitionPrefix? DefinitionExtensionKeyword*
/// ```
///
/// `SysML` 8.2.2.6.1. It is [`OccurrenceDefinitionPrefix`](crate::OccurrenceDefinitionPrefix)
/// without the `individual` group: only an occurrence may be individual, so
/// `individual attribute def A;` is an error, and the two are separate productions for
/// that reason (`definition_prefix`, `crates/sv2-syntax/src/parser/definition.rs`). In the
/// corpus there are 291, under `AttributeDefinition` (171) and `PortDefinition` (120), the
/// two definitions that are not occurrences.
///
/// **Almost always empty.** The node is built even when both slots are, and 284 of the 291
/// are. The other 7 write one slot each: `variation` (2) or a `#X` (5). An empty prefix is
/// zero width, and both accessors answer absence.
///
/// The node holds its slots and no token of its own. Both slots have kinds of their own
/// and no view yet, so they are returned raw until they do.
// node: DefinitionPrefix
pub struct DefinitionPrefix(SyntaxNode);

impl AstNode for DefinitionPrefix {
    const KIND: SyntaxKind = SyntaxKind::DefinitionPrefix;

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

impl DefinitionPrefix {
    /// The `BasicDefinitionPrefix`, `abstract` or `variation`, when one is written.
    ///
    /// A raw `SyntaxNode` of kind `BasicDefinitionPrefix`, because that kind has no view
    /// yet. Which keyword it is belongs to that node: ask it for its token.
    ///
    /// The first direct child of the kind. Taking the last, or searching descendants,
    /// cannot be observed: `tests/sweep.rs` asserts the basic prefix comes first if at all,
    /// and that nothing beneath a slot is itself a slot.
    pub fn basic_prefix(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::BasicDefinitionPrefix)
    }

    /// Each `DefinitionExtensionKeyword` written, a `#X` user-defined keyword, in source
    /// order. "It is also possible to include more than one user defined-keyword in a
    /// declaration" (7.27.4, receipt 0f2c5bd1). No corpus prefix writes two, so the order
    /// is pinned by `tests/positive.rs`. Reading descendants rather than children cannot
    /// be observed, because `tests/sweep.rs` asserts that nothing beneath a slot is a slot.
    ///
    /// Raw nodes of kind `DefinitionExtensionKeyword`, because that kind has no view yet.
    /// A word this prefix has no slot for is not reported here. `individual attribute def
    /// A;` puts `individual` in an `Error` beside the member, and this prefix answers
    /// empty (`tests/malformed/definition-prefix/individual-has-no-slot.sysml`).
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }
}
