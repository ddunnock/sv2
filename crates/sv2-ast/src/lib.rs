// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Typed accessors over the lossless syntax tree from `sv2-syntax`.
//!
//! This crate owns no data. Every accessor is a view over a CST node, so the tree
//! stays the single source of truth and the text stays authoritative (ADR-0001).

mod ast_node;
mod import;
mod namespace;

pub use crate::ast_node::AstNode;
pub use crate::import::Import;
pub use crate::namespace::RootNamespace;
