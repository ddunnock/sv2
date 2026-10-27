// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
#![allow(
    dead_code,
    reason = "this module is compiled into every test binary that declares \
              `mod simple_definitions;`; a helper used by only one of those binaries is \
              unused in the others"
)]
//! What the productions `simple_definition` builds share, read once for all of them.
//!
//! `crates/sv2-syntax/src/parser/definition.rs` builds eight productions with one method,
//! from the `SIMPLE_DEFINITIONS` table:
//!
//! ```text
//! <Kind>Definition = <Prefix> '<keyword>' 'def' Definition
//! ```
//!
//! They differ in the node kind, the first keyword, and whether the prefix is a
//! `DefinitionPrefix` (`attribute`, the one that is not an occurrence) or an
//! `OccurrenceDefinitionPrefix` (the other seven). Their accessors are separate types,
//! one file each in `src/`, because each is a view of one node kind (ADR-0025). What
//! their tests assert is the same, so it is written once, here, against a [`Production`]
//! that names those three differences and a reader that calls the typed accessors.
//!
//! `PortDefinition` shares them too, though `port_definition`
//! (`crates/sv2-syntax/src/parser/connection.rs`) builds it. Its tree is an attribute
//! definition's with one more child after the `Definition`, a
//! `ConjugatedPortDefinitionMember` holding no token, which [`Production::tail`]
//! names. Its recovery is theirs: `at_port_definition` begins one only on `port def`, and
//! the body's `Definition` is read by the same `definition`.
//!
//! The per-kind entry points stay per kind: `tests/sweep.rs` keeps an
//! `assert_<kind>_accessor` for each, which `scripts/ast_coverage.py` requires by name,
//! and each malformed case keeps its own test function and its own fixture directory.

use sv2_ast::{
    AllocationDefinition, AstNode, AttributeDefinition, ConcernDefinition, ConnectionDefinition,
    Definition, FlowDefinition, ItemDefinition, OccurrenceDefinition, PartDefinition,
    PortDefinition, RenderingDefinition, RequirementDefinition,
};
use sv2_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// One production `simple_definition` builds: `<prefix> '<keyword>' 'def' Definition`.
pub(crate) struct Production {
    /// The node kind the production builds, and the view's `KIND`.
    pub(crate) node: SyntaxKind,
    /// `DefinitionPrefix` or `OccurrenceDefinitionPrefix`.
    pub(crate) prefix: SyntaxKind,
    /// The first keyword's token kind.
    pub(crate) keyword: SyntaxKind,
    /// The first keyword as written.
    pub(crate) keyword_text: &'static str,
    /// The usage the same keyword opens when no `def` follows it, which is what
    /// `at_simple_definition` reads `<keyword> A def B;` as.
    pub(crate) usage: SyntaxKind,
    /// The node kinds the production holds after `def`, in order: `Definition` for the eight
    /// `simple_definition` builds; `Definition` and the zero-width
    /// `ConjugatedPortDefinitionMember` for a port definition; and `DefinitionDeclaration`
    /// and `RequirementBody` for a requirement or concern definition, which have no
    /// `Definition` node.
    pub(crate) tail: &'static [SyntaxKind],
    /// The production read through its typed view, or `None` where the view's `cast`
    /// refuses the node.
    pub(crate) read: fn(&SyntaxNode) -> Option<Read>,
}

/// What a view of one production answers, each child as the view returns it.
pub(crate) struct Read {
    /// `syntax()`: the node the view was cast from.
    pub(crate) syntax: SyntaxNode,
    /// `prefix()`, as the node it wraps.
    pub(crate) prefix: Option<SyntaxNode>,
    /// The first keyword's accessor: `part_keyword()`, `attribute_keyword()`, and so on.
    pub(crate) keyword: Option<SyntaxToken>,
    /// `def_keyword()`.
    pub(crate) def_keyword: Option<SyntaxToken>,
    /// What the view's accessors answer for the [`Production::tail`] kinds, in order, as
    /// the nodes they wrap: `definition()`, or `declaration()` and `body()`. It stops short
    /// of a tail kind with no accessor yet, a port's conjugate member.
    pub(crate) tail: Vec<Option<SyntaxNode>>,
    /// The declaration, through `definition()` where the production has one.
    pub(crate) declaration: Option<SyntaxNode>,
    /// The body, through `definition()` where the production has one.
    pub(crate) body: Option<SyntaxNode>,
}

/// The tail of the eight `simple_definition` builds: one `Definition`.
const DEFINITION: &[SyntaxKind] = &[SyntaxKind::Definition];

impl Read {
    /// A view whose tail is one `Definition`, read through it.
    fn with_definition(
        syntax: SyntaxNode,
        prefix: Option<SyntaxNode>,
        keyword: Option<SyntaxToken>,
        def_keyword: Option<SyntaxToken>,
        definition: Option<Definition>,
    ) -> Self {
        let declaration = definition.as_ref().and_then(Definition::declaration);
        let body = definition.as_ref().and_then(Definition::body);
        Self {
            syntax,
            prefix,
            keyword,
            def_keyword,
            tail: vec![definition.map(|definition| definition.syntax().clone())],
            declaration,
            body,
        }
    }
}

/// `AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition` (`SysML` 8.2.2.7).
pub(crate) const ATTRIBUTE: Production = Production {
    node: SyntaxKind::AttributeDefinition,
    prefix: SyntaxKind::DefinitionPrefix,
    keyword: SyntaxKind::KwAttribute,
    keyword_text: "attribute",
    usage: SyntaxKind::AttributeUsage,
    tail: DEFINITION,
    read: read_attribute,
};

/// `OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' Definition`
/// (`SysML` 8.2.2.9.1).
pub(crate) const OCCURRENCE: Production = Production {
    node: SyntaxKind::OccurrenceDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwOccurrence,
    keyword_text: "occurrence",
    usage: SyntaxKind::OccurrenceUsage,
    tail: DEFINITION,
    read: read_occurrence,
};

/// `ItemDefinition = OccurrenceDefinitionPrefix 'item' 'def' Definition` (`SysML` 8.2.2.10).
pub(crate) const ITEM: Production = Production {
    node: SyntaxKind::ItemDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwItem,
    keyword_text: "item",
    usage: SyntaxKind::ItemUsage,
    tail: DEFINITION,
    read: read_item,
};

/// `PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition` (`SysML`
/// 8.2.2.11).
pub(crate) const PART: Production = Production {
    node: SyntaxKind::PartDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwPart,
    keyword_text: "part",
    usage: SyntaxKind::PartUsage,
    tail: DEFINITION,
    read: read_part,
};

/// `ConnectionDefinition = OccurrenceDefinitionPrefix 'connection' 'def' Definition` (`SysML`
/// 8.2.2.13.1).
pub(crate) const CONNECTION: Production = Production {
    node: SyntaxKind::ConnectionDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwConnection,
    keyword_text: "connection",
    usage: SyntaxKind::ConnectionUsage,
    tail: DEFINITION,
    read: read_connection,
};

/// `FlowDefinition = OccurrenceDefinitionPrefix 'flow' 'def' Definition` (`SysML`
/// 8.2.2.16).
pub(crate) const FLOW: Production = Production {
    node: SyntaxKind::FlowDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwFlow,
    keyword_text: "flow",
    usage: SyntaxKind::FlowUsage,
    tail: DEFINITION,
    read: read_flow,
};

/// `AllocationDefinition = OccurrenceDefinitionPrefix 'allocation' 'def' Definition` (`SysML`
/// 8.2.2.15).
pub(crate) const ALLOCATION: Production = Production {
    node: SyntaxKind::AllocationDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwAllocation,
    keyword_text: "allocation",
    usage: SyntaxKind::AllocationUsage,
    tail: DEFINITION,
    read: read_allocation,
};

/// `RenderingDefinition = OccurrenceDefinitionPrefix 'rendering' 'def' Definition` (`SysML`
/// 8.2.2.26.4).
pub(crate) const RENDERING: Production = Production {
    node: SyntaxKind::RenderingDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwRendering,
    keyword_text: "rendering",
    usage: SyntaxKind::RenderingUsage,
    tail: DEFINITION,
    read: read_rendering,
};

/// `PortDefinition = DefinitionPrefix 'port' 'def' Definition ownedRelationship +=
/// ConjugatedPortDefinitionMember` (`SysML` 8.2.2.12). Not one of `simple_definition`'s:
/// `port_definition` builds it, and its tree differs from theirs only in the trailing
/// member, which holds no token.
pub(crate) const PORT: Production = Production {
    node: SyntaxKind::PortDefinition,
    prefix: SyntaxKind::DefinitionPrefix,
    keyword: SyntaxKind::KwPort,
    keyword_text: "port",
    usage: SyntaxKind::PortUsage,
    tail: &[
        SyntaxKind::Definition,
        SyntaxKind::ConjugatedPortDefinitionMember,
    ],
    read: read_port,
};

/// `RequirementDefinition = OccurrenceDefinitionPrefix 'requirement' 'def'
/// DefinitionDeclaration RequirementBody` (`SysML` 8.2.2.21.1). Not one of
/// `simple_definition`'s: `requirement_definition` builds it, with no `Definition` node.
pub(crate) const REQUIREMENT: Production = Production {
    node: SyntaxKind::RequirementDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwRequirement,
    keyword_text: "requirement",
    usage: SyntaxKind::RequirementUsage,
    tail: &[
        SyntaxKind::DefinitionDeclaration,
        SyntaxKind::RequirementBody,
    ],
    read: read_requirement,
};

/// `ConcernDefinition = OccurrenceDefinitionPrefix 'concern' 'def' DefinitionDeclaration
/// RequirementBody` (`SysML` 8.2.2.21.3): a requirement definition's tail, with the kind
/// keyword `concern`, built by `concern_definition`.
pub(crate) const CONCERN: Production = Production {
    node: SyntaxKind::ConcernDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwConcern,
    keyword_text: "concern",
    usage: SyntaxKind::ConcernUsage,
    tail: &[
        SyntaxKind::DefinitionDeclaration,
        SyntaxKind::RequirementBody,
    ],
    read: read_concern,
};

fn read_attribute(node: &SyntaxNode) -> Option<Read> {
    AttributeDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.attribute_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_occurrence(node: &SyntaxNode) -> Option<Read> {
    OccurrenceDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.occurrence_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_item(node: &SyntaxNode) -> Option<Read> {
    ItemDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.item_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_part(node: &SyntaxNode) -> Option<Read> {
    PartDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.part_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_connection(node: &SyntaxNode) -> Option<Read> {
    ConnectionDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.connection_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_flow(node: &SyntaxNode) -> Option<Read> {
    FlowDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.flow_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_allocation(node: &SyntaxNode) -> Option<Read> {
    AllocationDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.allocation_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_rendering(node: &SyntaxNode) -> Option<Read> {
    RenderingDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.rendering_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_port(node: &SyntaxNode) -> Option<Read> {
    PortDefinition::cast(node.clone()).map(|view| {
        Read::with_definition(
            view.syntax().clone(),
            view.prefix().map(|prefix| prefix.syntax().clone()),
            view.port_keyword(),
            view.def_keyword(),
            view.definition(),
        )
    })
}

fn read_requirement(node: &SyntaxNode) -> Option<Read> {
    RequirementDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.requirement_keyword(),
        def_keyword: view.def_keyword(),
        tail: vec![view.declaration(), view.body()],
        declaration: view.declaration(),
        body: view.body(),
    })
}

fn read_concern(node: &SyntaxNode) -> Option<Read> {
    ConcernDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.concern_keyword(),
        def_keyword: view.def_keyword(),
        tail: vec![view.declaration(), view.body()],
        declaration: view.declaration(),
        body: view.body(),
    })
}

/// Every instance of `production` in `root`, in document order, read.
pub(crate) fn every(production: &Production, root: &SyntaxNode) -> Vec<Read> {
    root.descendants()
        .filter_map(|node| (production.read)(&node))
        .collect()
}

/// What a malformed case compares: the two keywords as written, and the declaration and
/// body, empty where absent, when the first of the tail is there.
pub(crate) type Summary = (Option<String>, Option<String>, Option<(String, String)>);

/// `read`, as text.
pub(crate) fn summary(read: &Read) -> Summary {
    let text = |node: &Option<SyntaxNode>| {
        node.as_ref()
            .map(|node| node.text().to_string())
            .unwrap_or_default()
    };
    (
        read.keyword.as_ref().map(|token| token.text().to_owned()),
        read.def_keyword
            .as_ref()
            .map(|token| token.text().to_owned()),
        read.tail
            .first()
            .and_then(Option::as_ref)
            .map(|_| (text(&read.declaration), text(&read.body))),
    )
}

/// A whole instance of `production` declaring `name`, closed by `;`.
pub(crate) fn whole(production: &Production, name: &str) -> Summary {
    (
        Some(production.keyword_text.to_owned()),
        Some("def".to_owned()),
        Some((name.to_owned(), ";".to_owned())),
    )
}
