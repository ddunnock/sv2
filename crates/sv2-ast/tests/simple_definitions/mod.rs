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
//! The per-kind entry points stay per kind: `tests/sweep.rs` keeps an
//! `assert_<kind>_accessor` for each, which `scripts/ast_coverage.py` requires by name,
//! and each malformed case keeps its own test function and its own fixture directory.

use sv2_ast::{
    AstNode, AttributeDefinition, ConnectionDefinition, Definition, ItemDefinition,
    OccurrenceDefinition, PartDefinition,
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
    /// `definition()`.
    pub(crate) definition: Option<Definition>,
}

/// `AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition` (`SysML` 8.2.2.7).
pub(crate) const ATTRIBUTE: Production = Production {
    node: SyntaxKind::AttributeDefinition,
    prefix: SyntaxKind::DefinitionPrefix,
    keyword: SyntaxKind::KwAttribute,
    keyword_text: "attribute",
    usage: SyntaxKind::AttributeUsage,
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
    read: read_occurrence,
};

/// `ItemDefinition = OccurrenceDefinitionPrefix 'item' 'def' Definition` (`SysML` 8.2.2.10).
pub(crate) const ITEM: Production = Production {
    node: SyntaxKind::ItemDefinition,
    prefix: SyntaxKind::OccurrenceDefinitionPrefix,
    keyword: SyntaxKind::KwItem,
    keyword_text: "item",
    usage: SyntaxKind::ItemUsage,
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
    read: read_connection,
};

fn read_attribute(node: &SyntaxNode) -> Option<Read> {
    AttributeDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.attribute_keyword(),
        def_keyword: view.def_keyword(),
        definition: view.definition(),
    })
}

fn read_occurrence(node: &SyntaxNode) -> Option<Read> {
    OccurrenceDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.occurrence_keyword(),
        def_keyword: view.def_keyword(),
        definition: view.definition(),
    })
}

fn read_item(node: &SyntaxNode) -> Option<Read> {
    ItemDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.item_keyword(),
        def_keyword: view.def_keyword(),
        definition: view.definition(),
    })
}

fn read_part(node: &SyntaxNode) -> Option<Read> {
    PartDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.part_keyword(),
        def_keyword: view.def_keyword(),
        definition: view.definition(),
    })
}

fn read_connection(node: &SyntaxNode) -> Option<Read> {
    ConnectionDefinition::cast(node.clone()).map(|view| Read {
        syntax: view.syntax().clone(),
        prefix: view.prefix().map(|prefix| prefix.syntax().clone()),
        keyword: view.connection_keyword(),
        def_keyword: view.def_keyword(),
        definition: view.definition(),
    })
}

/// Every instance of `production` in `root`, in document order, read.
pub(crate) fn every(production: &Production, root: &SyntaxNode) -> Vec<Read> {
    root.descendants()
        .filter_map(|node| (production.read)(&node))
        .collect()
}

/// What a malformed case compares: the two keywords as written, and the declaration and
/// body the `Definition` holds, empty where absent.
pub(crate) type Summary = (Option<String>, Option<String>, Option<(String, String)>);

/// `read`, as text.
pub(crate) fn summary(read: &Read) -> Summary {
    (
        read.keyword.as_ref().map(|token| token.text().to_owned()),
        read.def_keyword
            .as_ref()
            .map(|token| token.text().to_owned()),
        read.definition.as_ref().map(|definition| {
            (
                definition
                    .declaration()
                    .map(|declaration| declaration.text().to_string())
                    .unwrap_or_default(),
                definition
                    .body()
                    .map(|body| body.text().to_string())
                    .unwrap_or_default(),
            )
        }),
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
