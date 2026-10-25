// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! What an import imports: one of the two alternatives, wrapped.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The imported thing, as written: a membership or a whole namespace.
///
/// ```text
/// ImportDeclaration = MembershipImport | NamespaceImport
///
/// MembershipImport = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
/// NamespaceImport  = importedNamespace  = [QualifiedName] '::' '*'
///                                        ( '::' isRecursive ?= '**' )?
/// ```
///
/// `KerML` 8.2.3.4.2 and `SysML` 8.2.2.5.1.
///
/// **A pure wrapper, and measurably so**: across this repository's 610 trees there are
/// 756 of these nodes, and every one holds exactly one child node and **no token of its
/// own**. The separators, the `*` and the `**` all belong to the alternative inside, so
/// nothing here goes looking for them — doing so would be reaching across a production
/// boundary, and it would not find them anyway. `tests/sweep.rs` asserts both halves.
///
/// Its parents are `Import` (744), `FilterPackageImport` (11) and `FilterPackage` (1):
/// `SysML`'s `MembershipExpose` and `NamespaceExpose` wrap the same reader (8.2.2.26.2),
/// which is why `imported` in the parser is shared rather than inlined.
// node: ImportDeclaration
pub struct ImportDeclaration(SyntaxNode);

impl AstNode for ImportDeclaration {
    const KIND: SyntaxKind = SyntaxKind::ImportDeclaration;

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

impl ImportDeclaration {
    /// The alternative that matched: a `MembershipImport` or a `NamespaceImport` node.
    ///
    /// **Untyped for a different reason than the other raw accessor in this crate.**
    /// [`ElementFilterMember::filter`](crate::ElementFilterMember::filter) is raw because
    /// `OwnedExpression` builds no node at all. Here both alternatives *do* have node
    /// kinds of their own, so the delegation rule in `.claude/rules/ast.md` would ask for
    /// a typed view — but the child is one of **two** kinds, and `AstNode` declares one
    /// `KIND` per view, so no typed signature admits both. An enum over the two would be
    /// a new abstraction the trait cannot express, and `.claude/rules/ast.md` rules it
    /// out as invented before any alternative had an accessor. Instead the caller casts what this returns —
    /// [`MembershipImport::cast`](crate::MembershipImport) for the first alternative,
    /// [`NamespaceImport::cast`](crate::NamespaceImport) for the second — and this
    /// signature does not change as the alternatives gain views. Both have one now, and
    /// the enum question is deferred for the whole crate, not answered here (decision
    /// `ast-alternative-views`).
    ///
    /// So the returned node's `kind()` **is** the discriminator, and it is the honest one:
    /// what makes an import a `NamespaceImport` is the `::*`, not the presence of stars.
    /// `Q::**` contains no `::*` and is therefore a `MembershipImport` with `isRecursive`
    /// set — a recursive import of the membership, not of the namespace. Anything deciding
    /// the alternative by looking for a `*` character gets that case backwards, and
    /// `Imports.kerml` line 33 is the pinned input that says so
    /// (`tests/positive.rs::a_double_star_without_a_single_one_is_a_membership_import`).
    ///
    /// `Some` is not evidence that anything was imported, and neither is width. The node
    /// is opened before the name is sought, so `public import ;` leaves this returning a
    /// zero-width `MembershipImport` whose `QualifiedName` has no segments
    /// (`tests/malformed/import/declaration-names-nothing.sysml`), while
    /// `public import A::;` leaves a declaration WIDER than the name it carries, holding a
    /// complete-looking `MembershipImport` of `A` that nothing in the tree marks as
    /// damaged (`tests/malformed/import-declaration/trailing-separator.sysml`). In both,
    /// `Parse::errors()` is the only signal. `None` would mean the parser stopped building
    /// the checkpoint node, which no input does.
    ///
    /// Taking the **last** child instead of the first is an equivalent mutation, under
    /// the sweep's claim that a declaration wraps exactly one alternative — the same case
    /// `Import::child` records. Of seven mutations tried, five were caught, including
    /// both of the ones worth worrying about: reaching a grandchild, which would hand
    /// back the `QualifiedName` from inside the alternative, and reporting only namespace
    /// imports, which would make `Q::**` disappear. The seventh,
    /// `descendants().nth(1)`, was a badly chosen mutation rather than a survivor worth
    /// recording: `descendants()` yields the node itself first, so that expression
    /// denotes the first child by construction and states no defect at all.
    pub fn imported(&self) -> Option<SyntaxNode> {
        self.0.children().next()
    }
}
