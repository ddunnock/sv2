// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Tauri's build step: reads `tauri.conf.json` and `capabilities/`.
//!
//! The app's commands are declared here so that Tauri generates an `allow-<command>`
//! permission for each and refuses any command a capability does not grant. Without
//! this list every registered command is callable from every window, and the
//! capability file could not mirror the registry (STD-004-TS §2 rule 3).

/// `app/src/contract/registry.ts`'s commands, by their Rust names.
const COMMANDS: &[&str] = &[
    "workspace",
    "views",
    "element_detail",
    "view_layout",
    "file_text",
];

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )?;
    Ok(())
}
