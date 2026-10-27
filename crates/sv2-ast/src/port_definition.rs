// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A port definition: its prefix, its `port def` keywords and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;
use crate::definition_prefix::DefinitionPrefix;

/// A port definition.
///
/// ```text
/// PortDefinition = DefinitionPrefix 'port' 'def' Definition
///                  ownedRelationship += ConjugatedPortDefinitionMember
/// ```
///
/// `SysML` 8.2.2.12. Built by `port_definition`
/// (`crates/sv2-syntax/src/parser/connection.rs`), not by `simple_definition`: it reads
/// the same four elements as [`AttributeDefinition`](crate::AttributeDefinition), a
/// [`DefinitionPrefix`] because a port is not an occurrence, and then builds a
/// `ConjugatedPortDefinitionMember`, three nested nodes and no token, for the conjugate
/// every port definition implicitly declares (`port def P;` gives you `~P`). 120 in the
/// corpus, all under `PackageMember`.
///
/// **Both keywords are always there.** `at_port_definition` begins a port definition only
/// on `port` followed by `def`, so `port A def B;` is a `PortUsage` and builds no port
/// definition at all (`tests/malformed/port-definition/def-missing.sysml`). A missing
/// `def` is therefore a missing node, never a node missing its keyword, and the keyword
/// accessors answer `None` on no tree this parser builds.
///
/// The prefix and the definition have views, so both are returned typed. The keywords are
/// this node's own tokens. The conjugate member has no view yet, and no accessor here: it
/// gets one with its view, rather than a raw node now that every caller would have to
/// unlearn. The sweep asserts it is there, last, on every port definition.
// node: PortDefinition
pub struct PortDefinition(SyntaxNode);

impl AstNode for PortDefinition {
    const KIND: SyntaxKind = SyntaxKind::PortDefinition;

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

impl PortDefinition {
    /// The `DefinitionPrefix`: `abstract` or `variation`, and the user-defined keywords.
    /// Built even when nothing is written, so usually present and zero width
    /// (`DefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<DefinitionPrefix> {
        self.0.children().find_map(DefinitionPrefix::cast)
    }

    /// The `port` keyword.
    pub fn port_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwPort)
    }

    /// The `def` keyword: the one written straight after `port`. A second `def` is not
    /// this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/port-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `Definition`: the declaration and the body.
    ///
    /// Read from the direct children. Two mutations cannot be observed, each under a
    /// `tests/sweep.rs` assertion. Taking the last is equivalent because the definition is
    /// the node's only child of the kind. Reading descendants is equivalent because the
    /// prefix, the one element before it in preorder, is asserted to hold no `Definition`;
    /// the conjugate member comes after it.
    pub fn definition(&self) -> Option<Definition> {
        self.0.children().find_map(Definition::cast)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last token of the kind is equivalent because
    /// the sweep asserts the node's elements are exactly `DefinitionPrefix 'port' 'def'
    /// Definition ConjugatedPortDefinitionMember`, one of each. Searching descendant
    /// tokens is equivalent because it also asserts the prefix, the only element before
    /// the keywords, holds neither; a port definition nested in the body comes after them
    /// in preorder. Taking the first own token whatever its kind is not equivalent, and is
    /// caught: it answers `port` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
