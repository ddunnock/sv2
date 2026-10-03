// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Diagnostics and recovery: what the parser reports when text does not match, the
//! `PARSE-DEVIATION` notes of ADR-0022, and the statement-bounded recovery that keeps one
//! bad token from flooding the file with errors.

use text_size::{TextRange, TextSize};

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::generated::kinds::SyntaxKind;
use crate::lexer::Token;
use crate::parser::lookahead::keyword;
use crate::parser::{MAX_DEPTH, Parser};

impl Parser<'_> {
    /// Consume the next token as `kind`, or record an error without consuming.
    pub(super) fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if self.at(kind) {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    /// Consume `text` as a keyword, or record an error without consuming.
    pub(super) fn expect_keyword(&mut self, text: &str) {
        if self.at_keyword(text) {
            self.bump_as(keyword(text).unwrap_or(SyntaxKind::BasicName));
        } else {
            self.error_expected(&format!("`{text}`"));
        }
    }

    /// Consume the next token as a NAME, or record an error without consuming.
    pub(super) fn expect_name(&mut self, what: &str) {
        if self.at_name() {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    /// Report that the input is nested deeper than the parser will recurse.
    ///
    /// Said once rather than once per level: the limit is a property of the input as
    /// a whole, and one diagnostic per level would bury every other one.
    pub(super) fn report_too_deep(&mut self) {
        if self.depth_reported {
            return;
        }
        self.depth_reported = true;
        let range = self.here();
        self.emit(
            DiagnosticCode::TooDeeplyNested,
            range,
            format!("nested deeper than {MAX_DEPTH} levels"),
        );
    }

    pub(super) fn error_expected(&mut self, what: &str) {
        let found = self
            .peek()
            .map_or("end of file", |token| self.text_of(token));
        let message = format!("expected {what}, found `{found}`");
        let range = self.here();
        self.emit(DiagnosticCode::Expected, range, message);
    }

    /// Record that the text at the next meaningful token is admitted only by the register
    /// entry `entry` (ADR-0022).
    ///
    /// Every call site carries a `// deviation: <entry>` marker, which
    /// `scripts/check_deviation_sites.py` checks against the register: the entry must exist
    /// and must depart from the specification. A site calls this only where the text uses
    /// what the deviation adds, never merely where the deviation could apply.
    pub(super) fn note_deviation(&mut self, entry: &str, what: &str) {
        let range = self.here();
        self.deviations.push(Diagnostic::new(
            DiagnosticCode::Deviation,
            range,
            format!("{what}: admitted by deviation {entry}, not by the specification's BNF"),
        ));
    }

    /// Record one diagnostic.
    ///
    /// Every diagnostic this parser raises goes through here, so that the range is
    /// never forgotten: a `Diagnostic` without one cannot be constructed, which is what
    /// keeps "underline the offending token" from being a thing a caller has to guess.
    ///
    /// ONE DIAGNOSTIC PER START OFFSET. A repeat at the offset the last one starts at is
    /// dropped, keeping the FIRST raiser, which is the innermost: the production that
    /// could not read the token reports before the enclosing ones that then give up. Four
    /// reports about one `new` tell a reader nothing the first does not, and a token this
    /// parser cannot read is one defect however many nested productions fail on it.
    ///
    /// The range is compared, not the code: two codes at one offset (`PARSE-EXPECTED` and
    /// `PARSE-UNEXPECTED` both fire on an unreadable token) are the same defect described
    /// twice. Only the LAST diagnostic is compared, as Ruff's `add_error` does, because
    /// recovery moves forward: an offset that repeats does so consecutively, and comparing
    /// against every diagnostic would make this quadratic on a file full of errors.
    ///
    /// Nothing is lost that a caller needs: the tree still carries every byte
    /// (invariant 1), the element still enters the IR with its diagnostics (invariant 2,
    /// ADR-0002), and a defect at a NEW offset is never dropped.
    pub(super) fn emit(&mut self, code: DiagnosticCode, range: TextRange, message: String) {
        if self
            .errors
            .last()
            .is_some_and(|last| last.range().start() == range.start())
        {
            return;
        }
        self.errors.push(Diagnostic::new(code, range, message));
    }

    /// The range a diagnostic about what comes next is about.
    ///
    /// The next non-trivia token, or an empty range at the end of the source when there
    /// is none. Empty is the honest answer for "expected `}`, found end of file": it is
    /// about a position rather than about any bytes, and it still has to be pointed at.
    fn here(&self) -> TextRange {
        self.peek().map_or_else(
            || TextRange::empty(Self::size(self.source.len())),
            Self::range_of,
        )
    }

    /// One token's half-open byte range.
    pub(super) fn range_of(token: Token) -> TextRange {
        TextRange::new(Self::size(token.start), Self::size(token.end))
    }

    /// A byte offset as a `TextSize`, saturating rather than wrapping.
    ///
    /// `TextSize` is 32-bit. A source file large enough to overflow it is not one this
    /// parser has to place diagnostics in correctly, but it is one it must not panic on
    /// (invariant 3), so the conversion saturates and the position is merely wrong.
    fn size(offset: usize) -> TextSize {
        TextSize::new(u32::try_from(offset).unwrap_or(u32::MAX))
    }

    /// One token nothing accepts, wrapped so its bytes survive in the tree.
    pub(super) fn error_token(&mut self) {
        self.eat_trivia();
        let Some(token) = self.tokens.get(self.pos).copied() else {
            return;
        };
        let message = format!("unexpected `{}`", self.text_of(token));
        self.emit(DiagnosticCode::Unexpected, Self::range_of(token), message);
        self.start_node(SyntaxKind::Error);
        self.push(token, token.kind);
        self.finish_node();
    }

    /// Recover over text no item production accepts, as far as the statement goes.
    ///
    /// One `Error` node and ONE diagnostic for the whole run, where recovering a token at
    /// a time gave one of each per token: a line whose only defect was an unimplemented
    /// `new` reported 23 times, because each skipped token reported itself and each bare
    /// NAME in the middle of the run restarted a keywordless usage that then failed on the
    /// token after it.
    ///
    /// `SysML` has no newline terminator, so the boundary has to be written: a `;` at the
    /// depth recovery started from ends the statement and is taken; the enclosing `}` ends
    /// it and is LEFT for the body loop, which owns it; and a keyword that begins a member
    /// ends it too, so the valid item after a missing `;` is still read as an item rather
    /// than swallowed. A bare NAME does NOT end it, which is the whole point — that is the
    /// restart that cascaded.
    ///
    /// Braces are balanced while skipping, so a `;` or `}` inside a nested body does not
    /// end the outer statement. The first token is always taken, so recovery always
    /// advances and the loop that calls this cannot spin (invariant 3). Every token still
    /// enters the tree, so `parse(s).text() == s` holds over the skipped run as it did
    /// over the single tokens (invariant 1).
    pub(super) fn recover_statement(&mut self) {
        self.eat_trivia();
        let Some(first) = self.tokens.get(self.pos).copied() else {
            return;
        };
        let start = Self::range_of(first).start();
        let mut end = Self::range_of(first).end();
        self.start_node(SyntaxKind::Error);
        let mut depth: usize = 0;
        let mut taken = false;
        while let Some(token) = self.tokens.get(self.pos).copied() {
            if taken && depth == 0 && (token.kind == SyntaxKind::RBrace || self.at_member_keyword())
            {
                break;
            }
            match token.kind {
                SyntaxKind::LBrace => depth += 1,
                SyntaxKind::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            if !self.skippable(token.kind) {
                end = Self::range_of(token).end();
                taken = true;
            }
            self.push(token, token.kind);
            if depth == 0 && token.kind == SyntaxKind::Semicolon {
                break;
            }
        }
        self.finish_node();
        let message = format!("unexpected `{}`", self.text_of(first));
        self.emit(
            DiagnosticCode::Unexpected,
            TextRange::new(start, end),
            message,
        );
    }
}
