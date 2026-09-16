// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The `sv2` command line: argument handling and output, testable without a subprocess.

mod cli;
mod error;

pub use crate::cli::run;
pub use crate::error::{Coded, CommandError, ErrorCode};
