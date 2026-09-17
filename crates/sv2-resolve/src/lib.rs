// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Libraries, name resolution, and derived properties over the HIR from `sv2-hir`.
//!
//! Resolution is local and owned: no network, no language server protocol client, no
//! upstream resolver is consulted (ADR-0007). Validity gates writes, never reads, so an
//! element whose references do not resolve still enters the IR carrying its diagnostics
//! (ADR-0002).
//!
//! A stub: no resolution exists yet, so the crate has no public API.
