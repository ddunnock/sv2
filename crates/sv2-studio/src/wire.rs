// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The IPC wire types the webview reads: the Rust half of the contract.
//!
//! Each type mirrors a module in `app/src/contract/` field for field, and
//! STD-004-TS §4.2 fixes the conventions both halves follow: camelCase names,
//! unions tagged `kind`, `null` rather than an absent field. The TypeScript
//! half is authoritative for nothing — it parses what this sends — but it is
//! where each shape was first written down, so the doc comment on each type
//! names the TypeScript type it must match.

use serde::{Deserialize, Serialize};

/// Every reply: the data, or why there is none (`contract/availability.ts` `Answer`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Answer<T> {
    /// The core has an answer.
    Ready {
        /// The answer.
        data: T,
    },
    /// The core cannot answer, and says why.
    Unavailable {
        /// Why.
        reason: UnavailableReason,
    },
}

impl<T> Answer<T> {
    /// A `not-implemented` answer naming what is missing.
    pub(crate) fn not_implemented(capability: &str) -> Self {
        Self::Unavailable {
            reason: UnavailableReason::NotImplemented {
                capability: capability.to_owned(),
            },
        }
    }

    /// A `not-found` answer.
    pub(crate) fn not_found() -> Self {
        Self::Unavailable {
            reason: UnavailableReason::NotFound,
        }
    }
}

/// Why there is no answer (`contract/availability.ts` `UnavailableReason`).
///
/// Only the arms this crate can raise today. The TypeScript union also has
/// `no-workspace`, `opening` and `read-only`, which arrive with the code that
/// can be in those states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum UnavailableReason {
    /// The core does not do this yet.
    NotImplemented {
        /// What is missing, as a phrase: "resolving views".
        capability: String,
    },
    /// The subject does not exist.
    NotFound,
}

/// A grammar (`contract/file.ts` `Language`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    /// `.sysml`
    Sysml,
    /// `.kerml`
    Kerml,
}

/// Diagnostic counts by severity (`contract/file.ts` `DiagnosticCounts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub(crate) struct DiagnosticCounts {
    /// Errors.
    pub(crate) error: u32,
    /// Warnings.
    pub(crate) warning: u32,
    /// Informational diagnostics.
    pub(crate) info: u32,
}

/// One model file (`contract/file.ts` `WorkspaceFile`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WorkspaceFile {
    /// Relative to the workspace root, `/`-separated (the one form a path crosses in).
    pub(crate) path: String,
    /// The grammar that parsed it.
    pub(crate) language: Language,
    /// Its diagnostics, counted.
    pub(crate) diagnostics: DiagnosticCounts,
}

/// An open workspace (`contract/file.ts` `Workspace`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Workspace {
    /// What the tree's root row shows: the root directory's name.
    pub(crate) name: String,
    /// Every model file, sorted by path.
    pub(crate) files: Vec<WorkspaceFile>,
}

/// An open file on its way between windows (`contract/editor-window.ts` `EditorHandoff`).
///
/// `state` is the editor's own serialization of the document, undo history
/// included. This crate carries it and never reads it: it is the editor's,
/// and the editor that receives it falls back to `text` if it will not restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditorHandoff {
    /// The workspace path of the file being edited.
    pub(crate) path: String,
    /// The text as last edited, which may differ from the disk: nothing saves yet.
    pub(crate) text: String,
    /// The editor's state, opaque here.
    pub(crate) state: serde_json::Value,
}

/// A model file's text (`contract/file.ts` `FileText`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FileText {
    /// The workspace path it was asked for by.
    pub(crate) path: String,
    /// The text, exactly as on disk.
    pub(crate) text: String,
}
