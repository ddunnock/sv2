// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The IPC commands the webview calls: `app/src/contract/registry.ts`, in Rust.
//!
//! Each name is the registry's `command`, and each reply is an `Answer`. Two
//! are answered from the workspace today. The three that need the resolved
//! model answer `not-implemented`, because `sv2-resolve` is a stub — the
//! honest reply, and the same code path the real answer will take.

use serde::de::IgnoredAny;
use tauri::State;

use crate::wire::{Answer, FileText, Workspace};
use crate::workspace::WorkspaceRoot;

/// `workspace`: the Files tree.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
#[tauri::command]
pub(crate) fn workspace(root: State<'_, WorkspaceRoot>) -> Answer<Workspace> {
    Answer::Ready { data: root.scan() }
}

/// `file_text`: one file's text, for the editor.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
#[tauri::command]
pub(crate) fn file_text(root: State<'_, WorkspaceRoot>, path: String) -> Answer<FileText> {
    root.read(&path)
}

/// `views`: needs view usages and their kinds, which are resolver work.
#[tauri::command]
pub(crate) fn views() -> Answer<()> {
    Answer::not_implemented("resolving views")
}

/// `element_detail`: needs the resolved model. The handle is accepted and not
/// read: there is nothing yet to look it up in.
#[tauri::command]
pub(crate) fn element_detail(handle: IgnoredAny) -> Answer<()> {
    let IgnoredAny = handle;
    Answer::not_implemented("reading an element's specification")
}

/// `view_layout`: needs the layout sidecar reader (ADR-0017), not written yet.
#[tauri::command]
pub(crate) fn view_layout(view: IgnoredAny) -> Answer<()> {
    let IgnoredAny = view;
    Answer::not_implemented("diagram layout")
}
