// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The Tauri shell: the process that hosts the webview, the diagram renderer, and the
//! editor. What Tauri's default layout calls `src-tauri/`.
//!
//! A second binary, and deliberately one (ADR-0018). `sv2` is a batch command whose
//! whole contract is argument handling, one exit status per error code, and doing no
//! work before the request is known (STD-002-RS §3). A long-lived, event-driven webview
//! host satisfies none of that, so it gets its own entry point rather than a mode flag
//! on a command that would then have two shapes.
//!
//! What it must not become is a place where logic lives. Every binary in this workspace
//! is a shim: the model is read by `sv2-resolve` and below, and this crate wires a
//! window to it.
//!
//! A stub: nothing starts yet, so `run` says so and exits 2.

mod shell;

pub use crate::shell::run;
