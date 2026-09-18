// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The lossless syntax layer: a lexer and parser over `SysML` v2 and `KerML` text.
//!
//! The tree keeps every byte of the source, including whitespace, both comment
//! forms, and the author's spacing. `parse(s).text() == s` holds for every input,
//! valid or not (ADR-0004), because a graphical edit downstream must produce a
//! minimal text delta and cannot if the tree threw formatting away.
//!
//! Nothing here does I/O, and nothing here panics on any input.

mod diagnostic;
pub mod generated;
mod grammar;
mod language;
mod lexer;
mod offset;
mod parser;

pub use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use crate::generated::kinds::SyntaxKind;
pub use crate::grammar::Language;
pub use crate::offset::{LineCol, OffsetMap};
// Re-exported because it is in this crate's public signatures — `Diagnostic::range`
// returns one. A consumer that had to name `text_size` itself could end up on a
// different version of the type than the tree uses.
pub use crate::language::{Sv2Language, SyntaxElement, SyntaxNode, SyntaxToken};
pub use crate::lexer::{Token, is_trivia, tokenize};
pub use crate::parser::{Parse, infix_table_for_test, parse};
pub use text_size::{TextRange, TextSize};
