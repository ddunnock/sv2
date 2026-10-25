// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The prefix of an occurrence definition: abstract or variation, individual, and the
//! user-defined keywords.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The prefix every occurrence definition opens with.
///
/// ```text
/// OccurrenceDefinitionPrefix : OccurrenceDefinition =
///     BasicDefinitionPrefix?
///     ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
///     DefinitionExtensionKeyword*
/// ```
///
/// `SysML` 8.2.2.9.1. It is shared by every occurrence definition and their
/// specializations. In the corpus that is 1,020 prefixes under 19 parent kinds,
/// `PartDefinition` (544), `ActionDefinition` (121) and `ItemDefinition` (93) the most.
/// `AttributeDefinition` and the other non-occurrence definitions take a
/// `DefinitionPrefix` instead, and `individual def X` is an `IndividualDefinition`
/// (8.2.2.9.1), whose `individual` has no prefix around it.
///
/// **Almost always empty.** The node is built even when every slot is
/// (`occurrence_definition_prefix`, `crates/sv2-syntax/src/parser/definition.rs`), and 970
/// of the 1,020 are. An empty prefix is the ordinary shape, not a defect: it is zero width,
/// and every accessor below answers absence.
///
/// Its slot children all have kinds of their own and none has a view yet, so they are
/// returned raw until they do. `individual` is this node's own token.
// node: OccurrenceDefinitionPrefix
pub struct OccurrenceDefinitionPrefix(SyntaxNode);

impl AstNode for OccurrenceDefinitionPrefix {
    const KIND: SyntaxKind = SyntaxKind::OccurrenceDefinitionPrefix;

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

impl OccurrenceDefinitionPrefix {
    /// The `BasicDefinitionPrefix`, `abstract` or `variation`, when one is written.
    ///
    /// A raw `SyntaxNode` of kind `BasicDefinitionPrefix`, because that kind has no view
    /// yet. Which keyword it is belongs to that node: ask it for its token.
    pub fn basic_prefix(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::BasicDefinitionPrefix)
    }

    /// Whether `individual` is written in this prefix: `isIndividual`.
    ///
    /// Read from this node's own tokens. A `false` means the prefix has no `individual`,
    /// not that none was written near it. `individual abstract part def X;` puts its
    /// `individual` in an `Error` beside the member, and this prefix reports it absent
    /// (`tests/malformed/occurrence-definition-prefix/individual-before-basic-prefix.sysml`).
    ///
    /// Two mutations cannot be observed, and each names the `tests/sweep.rs` assertion that
    /// holds it up. Answering `multiplicity().is_some()` instead is equivalent because the
    /// sweep asserts the two come together. Answering "any token of its own" is equivalent
    /// because the slot-order assertion admits no other own token.
    pub fn is_individual(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::KwIndividual)
    }

    /// The `EmptyMultiplicityMember` written with `individual`: `ownedRelationship +=
    /// EmptyMultiplicityMember`. Present exactly when [`is_individual`](Self::is_individual)
    /// is true, which `tests/sweep.rs` asserts.
    ///
    /// It spans no text. It is the element the abstract syntax says an individual
    /// definition owns (`empty_multiplicity_member`,
    /// `crates/sv2-syntax/src/parser/multiplicity.rs`), so it is returned as a node,
    /// raw, because that kind has no view yet.
    pub fn multiplicity(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::EmptyMultiplicityMember)
    }

    /// Each `DefinitionExtensionKeyword` written, a `#X` user-defined keyword, in source
    /// order. "It is also possible to include more than one user defined-keyword in a
    /// declaration" (7.27.4, receipt 0f2c5bd1).
    ///
    /// Raw nodes of kind `DefinitionExtensionKeyword`, because that kind has no view yet.
    /// Each holds the `PrefixMetadataMember` that names the metadata.
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts that nothing beneath a slot is itself a slot. The order is the source order,
    /// and two keywords are pinned by `tests/positive.rs`, because no corpus prefix writes
    /// two.
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }

    /// The first direct child of `kind`. First, not only: the production states at most
    /// one of each, and `tests/sweep.rs` asserts the slots' order over every tree, which is
    /// what makes taking the last an equivalent mutation. Searching descendants is
    /// equivalent too, because the sweep asserts that nothing beneath a slot is a slot.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }
}
