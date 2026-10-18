// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Typed accessors over the lossless syntax tree from `sv2-syntax`.
//!
//! This crate owns no data. Every accessor is a view over a CST node, so the tree
//! stays the single source of truth and the text stays authoritative (ADR-0001).

mod alias_member;
mod ast_node;
mod element_filter_member;
mod import;
mod member_prefix;
mod namespace;
mod package_member;
mod qualified_name;

pub use crate::alias_member::AliasMember;
pub use crate::ast_node::AstNode;
pub use crate::element_filter_member::ElementFilterMember;
pub use crate::import::Import;
pub use crate::member_prefix::MemberPrefix;
pub use crate::namespace::RootNamespace;
pub use crate::package_member::PackageMember;
pub use crate::qualified_name::QualifiedName;
