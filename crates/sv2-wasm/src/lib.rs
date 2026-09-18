// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The `CodeMirror` adapter: a node buffer for the webview, and nothing else.
//!
//! ADR-0013 hands the editor its syntax tree from the same Rust parser the rest of the
//! workspace uses, rather than maintaining a second grammar in Lezer. This crate is the
//! glue for that, and is deliberately narrow: `Tree.build`, the `@lezer/common` parser
//! subclass, and the highlighting configuration are all JavaScript-side and duplicate
//! nothing here.
//!
//! It may reach `sv2-syntax` and nothing else, on purpose. Library-wide resolution runs
//! in the studio's backend, not in the webview (ADR-0013 RISK-013-4), and the dependency
//! graph is what enforces that rather than discipline — `deny.toml` carries the edge, so
//! a `sv2-resolve` dependency here fails `cargo deny check bans` (ADR-0018).
//!
//! Nothing may depend on this crate. It is an artifact the webview loads, built for
//! `wasm32-unknown-unknown`; linking it into the host binary would put a second parser
//! in the same process as the first.
//!
//! A stub: no adapter exists yet, so the crate has no public API.
