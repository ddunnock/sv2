// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Typed accessors over the lossless syntax tree from `sv2-syntax`.
//!
//! This crate owns no data. Every accessor is a view over a CST node, so the tree
//! stays the single source of truth and the text stays authoritative (ADR-0001).

mod alias_member;
mod allocation_definition;
mod ast_node;
mod attribute_definition;
mod concern_definition;
mod connection_definition;
mod definition;
mod definition_prefix;
mod element_filter_member;
mod flow_definition;
mod import;
mod import_declaration;
mod individual_definition;
mod item_definition;
mod library_package;
mod member_prefix;
mod membership_import;
mod namespace;
mod namespace_feature_member;
mod namespace_import;
mod non_feature_member;
mod occurrence_definition;
mod occurrence_definition_prefix;
mod owned_annotation;
mod package;
mod package_member;
mod part_definition;
mod port_definition;
mod qualified_name;
mod relationship_body;
mod rendering_definition;
mod requirement_definition;
mod visibility_indicator;

pub use crate::alias_member::AliasMember;
pub use crate::allocation_definition::AllocationDefinition;
pub use crate::ast_node::AstNode;
pub use crate::attribute_definition::AttributeDefinition;
pub use crate::concern_definition::ConcernDefinition;
pub use crate::connection_definition::ConnectionDefinition;
pub use crate::definition::Definition;
pub use crate::definition_prefix::DefinitionPrefix;
pub use crate::element_filter_member::ElementFilterMember;
pub use crate::flow_definition::FlowDefinition;
pub use crate::import::Import;
pub use crate::import_declaration::ImportDeclaration;
pub use crate::individual_definition::IndividualDefinition;
pub use crate::item_definition::ItemDefinition;
pub use crate::library_package::LibraryPackage;
pub use crate::member_prefix::MemberPrefix;
pub use crate::membership_import::MembershipImport;
pub use crate::namespace::RootNamespace;
pub use crate::namespace_feature_member::NamespaceFeatureMember;
pub use crate::namespace_import::NamespaceImport;
pub use crate::non_feature_member::NonFeatureMember;
pub use crate::occurrence_definition::OccurrenceDefinition;
pub use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;
pub use crate::owned_annotation::OwnedAnnotation;
pub use crate::package::Package;
pub use crate::package_member::PackageMember;
pub use crate::part_definition::PartDefinition;
pub use crate::port_definition::PortDefinition;
pub use crate::qualified_name::QualifiedName;
pub use crate::relationship_body::RelationshipBody;
pub use crate::rendering_definition::RenderingDefinition;
pub use crate::requirement_definition::RequirementDefinition;
pub use crate::visibility_indicator::VisibilityIndicator;
