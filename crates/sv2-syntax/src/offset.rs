// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Turning a byte offset into somewhere a person can look.
//!
//! **This module is the single designated offset boundary for the workspace.** A
//! `TextRange` is a pair of byte offsets, and every other way of naming a position —
//! line and column here, UTF-16 code units when a language server or a `CodeMirror`
//! document needs them (ADR-0013 RISK-013-2) — is derived here and nowhere else. A
//! second conversion anywhere, in a binary or in a test helper, defeats the test that
//! this one is right.
//!
//! Only [`OffsetMap::line_col`] exists today, because only a terminal consumes
//! positions today. The UTF-8 to UTF-16 half is deliberately absent rather than
//! written ahead of a caller: ADR-0013 is still `proposed`, and neither `sv2-lsp` nor
//! `sv2-wasm` exists. When it arrives it belongs in this file.

use text_size::TextSize;

/// A position as a person reads it: line and column, both counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    /// Line number, counted from 1.
    pub line: u32,
    /// Column, counted from 1 in **characters**, not bytes.
    ///
    /// Characters, because the number is for a person looking at a terminal, and a
    /// multi-byte identifier would otherwise report a column past where it is written.
    pub col: u32,
}

/// Where every line of one text begins.
///
/// Built once per file and asked many times, because a file with a hundred diagnostics
/// would otherwise be scanned a hundred times.
#[derive(Debug, Clone)]
pub struct OffsetMap {
    /// Byte offset of the first character of each line. Always starts with 0, so it is
    /// never empty and line 1 always exists, even for empty input.
    line_starts: Vec<TextSize>,
    /// The text this map was built from, kept so a column can be counted in characters.
    text: String,
}

impl OffsetMap {
    /// Index `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::new(0)];
        for (offset, _) in text.char_indices().filter(|(_, c)| *c == '\n') {
            // The line starts after the newline. `offset` is a char boundary and `\n`
            // is one byte, so the sum is a char boundary too.
            let next = offset.saturating_add(1);
            line_starts.push(Self::size(next));
        }
        Self {
            line_starts,
            text: text.to_owned(),
        }
    }

    /// `offset` as a byte count, saturating rather than wrapping.
    ///
    /// `TextSize` is 32-bit. A source file large enough to overflow it is not a thing
    /// this tool has to read correctly, but it is a thing it must not panic on
    /// (invariant 3), so the conversion saturates and the position is merely wrong.
    fn size(offset: usize) -> TextSize {
        TextSize::new(u32::try_from(offset).unwrap_or(u32::MAX))
    }

    /// The line and column `offset` falls on.
    ///
    /// An offset past the end of the text reports the last position, rather than
    /// failing: a diagnostic about the end of a truncated file is the normal case in an
    /// editor, not an error in the caller.
    #[must_use]
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        // The last line that starts at or before `offset`. `line_starts` is sorted and
        // begins with 0, so this never underflows.
        let index = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let start = self.line_starts.get(index).copied().unwrap_or_default();

        let from = usize::from(start);
        let to = usize::from(offset).min(self.text.len());
        // Characters rather than bytes, and `get` rather than a slice: an offset in the
        // middle of a multi-byte character must not panic.
        let col = self.text.get(from..to).map_or(0, |run| run.chars().count());

        LineCol {
            line: Self::size(index.saturating_add(1)).into(),
            col: Self::size(col.saturating_add(1)).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, offset: u32) -> (u32, u32) {
        let position = OffsetMap::new(text).line_col(TextSize::new(offset));
        (position.line, position.col)
    }

    #[test]
    fn the_first_character_is_line_one_column_one() {
        assert_eq!(at("package P;", 0), (1, 1));
        // Counted from 1, because that is what every editor and compiler prints.
        assert_eq!(at("package P;", 8), (1, 9));
    }

    #[test]
    fn a_newline_starts_the_next_line() {
        let text = "package P;\npackage Q;";
        assert_eq!(at(text, 10), (1, 11)); // the newline itself ends line 1
        assert_eq!(at(text, 11), (2, 1)); // the byte after it opens line 2
        assert_eq!(at(text, 19), (2, 9));
    }

    #[test]
    fn a_column_is_counted_in_characters_not_bytes() {
        // Four bytes, two characters, so what follows is column 3 and not column 5.
        let text = "éé x";
        assert_eq!(at(text, 4), (1, 3));
        assert_eq!(at(text, 5), (1, 4));
    }

    #[test]
    fn an_empty_text_still_has_a_line_one() {
        assert_eq!(at("", 0), (1, 1));
    }

    #[test]
    fn an_offset_past_the_end_reports_the_last_position() {
        // A diagnostic about the end of a truncated file is the editor's normal case.
        assert_eq!(at("ab", 99), (1, 3));
    }

    #[test]
    fn an_offset_inside_a_multi_byte_character_does_not_panic() {
        // Invariant 3. Byte 1 is inside the two-byte `é`; the answer is allowed to be
        // approximate, and is not allowed to be a panic.
        let _ = at("é", 1);
    }

    #[test]
    fn a_trailing_newline_opens_a_line_that_has_no_characters() {
        let text = "a\n";
        assert_eq!(at(text, 2), (2, 1));
    }
}
