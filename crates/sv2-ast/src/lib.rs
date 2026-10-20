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
mod import_declaration;
mod library_package;
mod member_prefix;
mod namespace;
mod namespace_feature_member;
mod non_feature_member;
mod package;
mod package_member;
mod qualified_name;
mod relationship_body;
mod visibility_indicator;

pub use crate::alias_member::AliasMember;
pub use crate::ast_node::AstNode;
pub use crate::element_filter_member::ElementFilterMember;
pub use crate::import::Import;
pub use crate::import_declaration::ImportDeclaration;
pub use crate::library_package::LibraryPackage;
pub use crate::member_prefix::MemberPrefix;
pub use crate::namespace::RootNamespace;
pub use crate::namespace_feature_member::NamespaceFeatureMember;
pub use crate::non_feature_member::NonFeatureMember;
pub use crate::package::Package;
pub use crate::package_member::PackageMember;
pub use crate::qualified_name::QualifiedName;
pub use crate::relationship_body::RelationshipBody;
pub use crate::visibility_indicator::VisibilityIndicator;
