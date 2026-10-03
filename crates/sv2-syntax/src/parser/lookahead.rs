// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Looking ahead: the meaningful-token index, `peek`, and the generic recognisers
//! every production uses to decide which alternative it is at, without consuming anything.

use crate::generated::kinds::{KEYWORDS, SyntaxKind};
use crate::lexer::{Token, is_trivia};
use crate::parser::Parser;

/// The kind the pinned token set gives `text`, or `None` if it names no keyword.
///
/// The keyword table is generated from the pinned token set, so asking it is what
/// keeps this parser tied to the pin rather than to a constant written out here.
///
/// The lookup returning `None` does not fail loudly — the caller falls back to
/// `BasicName`, because a parser that panics because a token set moved is worse than
/// one that mis-tags a node. What makes the fallback safe to have is
/// `every_keyword_this_parser_names_is_in_the_pinned_token_set` below: a keyword
/// leaving the token set fails the gate there, not silently at run time.
pub(super) fn keyword(text: &str) -> Option<SyntaxKind> {
    KEYWORDS
        .iter()
        .find(|(k, _)| *k == text)
        .map(|(_, kind)| *kind)
}

/// The three `VisibilityIndicator` keywords, in the order the specification
/// writes them (`SysML` 8.2.2.5.1). Looked up in the pinned token set like every
/// other keyword; this is only the list of which ones the production names.
pub(super) const VISIBILITY: [&str; 3] = ["public", "private", "protected"];

impl Parser<'_> {
    // -- looking ahead ----------------------------------------------------------

    /// The next non-trivia token, without consuming anything.
    pub(super) fn peek(&self) -> Option<Token> {
        self.peek_nth(0)
    }

    /// The `n`th non-trivia token from here, without consuming anything.
    ///
    /// `ImportDeclaration` needs two: `A::B` and `A::*` differ only after the `::`,
    /// and the `::` belongs to the `QualifiedName` in one and to the import in the
    /// other.
    pub(super) fn peek_nth(&self, n: usize) -> Option<Token> {
        let at = self
            .meaningful_index()
            .get(self.cursor_position().checked_add(n)?)?;
        self.tokens.get(*at).copied()
    }

    /// Where the cursor falls in `meaningful_index`: the position of the first meaningful
    /// token at or after `pos`.
    fn cursor_position(&self) -> usize {
        let (pos, significant, position) = self.cursor.get();
        if pos == self.pos && significant == self.comments_significant {
            return position;
        }
        let position = self.meaningful_index().partition_point(|&i| i < self.pos);
        self.cursor
            .set((self.pos, self.comments_significant, position));
        position
    }

    /// The meaningful-token index for the current comment mode: the tokens `skippable`
    /// does not skip.
    fn meaningful_index(&self) -> &[usize] {
        if self.comments_significant {
            &self.meaningful_with_comments
        } else {
            &self.meaningful
        }
    }

    /// Whether `kind` is trivia in the current context, and so skipped by lookahead.
    pub(super) fn skippable(&self, kind: SyntaxKind) -> bool {
        is_trivia(kind) && !(self.comments_significant && kind == SyntaxKind::RegularComment)
    }

    /// Run `production` with `REGULAR_COMMENT` treated as a token, then restore.
    pub(super) fn with_significant_comments(&mut self, production: impl FnOnce(&mut Self)) {
        let outer = self.comments_significant;
        self.comments_significant = true;
        production(self);
        self.comments_significant = outer;
    }

    pub(super) fn at(&self, kind: SyntaxKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// Whether the next meaningful token is a NAME.
    ///
    /// NAME is `BASIC_NAME` | `UNRESTRICTED_NAME` (`KerML` 8.2.2.3).
    ///
    /// A reserved word is excluded. `KerML` 8.2.2.6: "a reserved keyword is a token
    /// that has the lexical structure of a basic name but cannot actually be used as a
    /// basic name". The lexer cannot make that distinction, because `package` and
    /// `Vehicle` are the same token shape; the pinned keyword table is what separates
    /// them, and asking it here is what keeps `package package;` from declaring a
    /// package named `package`.
    pub(super) fn at_name(&self) -> bool {
        self.peek().is_some_and(|token| self.is_name(token))
    }

    /// Whether `token` is a NAME, asked of any token rather than only the next one.
    pub(super) fn is_name(&self, token: Token) -> bool {
        match token.kind {
            SyntaxKind::UnrestrictedName => true,
            SyntaxKind::BasicName => keyword(self.text_of(token)).is_none(),
            _ => false,
        }
    }

    /// Whether a `VisibilityIndicator` starts here (`SysML` 8.2.2.5.1).
    pub(super) fn at_visibility(&self) -> bool {
        VISIBILITY.iter().any(|word| self.at_keyword(word))
    }

    /// Whether the next meaningful token is this keyword.
    pub(super) fn at_keyword(&self, text: &str) -> bool {
        self.nth_is_keyword(0, text)
    }

    /// Whether the `n`th meaningful token from here is this keyword.
    pub(super) fn nth_is_keyword(&self, n: usize, text: &str) -> bool {
        self.peek_nth(n)
            .is_some_and(|token| token.kind == SyntaxKind::BasicName && self.text_of(token) == text)
    }

    /// Whether an `EndUsagePrefix`'s kind keyword stands at or after the `k`th token,
    /// before the end declaration's `;`, brace or `=`.
    pub(super) fn kind_follows(&self, k: usize) -> bool {
        let mut k = k;
        while let Some(token) = self.peek_nth(k) {
            if matches!(
                token.kind,
                SyntaxKind::Semicolon | SyntaxKind::LBrace | SyntaxKind::RBrace | SyntaxKind::Eq
            ) {
                return false;
            }
            if self.at_end_kind(k) {
                return true;
            }
            k += 1;
        }
        false
    }

    pub(super) fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    /// Whether the `n`th meaningful token is a NAME.
    pub(super) fn nth_is_name(&self, n: usize) -> bool {
        self.peek_nth(n).is_some_and(|token| self.is_name(token))
    }

    /// Consume whichever of `words` is written here, or nothing.
    ///
    /// An alternation of keyword flags: taking one forecloses the others, which is what
    /// leaves the second word of `composite portion` for the caller to report.
    pub(super) fn eat_one_of(&mut self, words: &[&str]) {
        if let Some(word) = words.iter().find(|word| self.at_keyword(word)) {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        }
    }

    /// Consume `text` if it is written here, leaving the position alone if it is not.
    pub(super) fn eat_optional_keyword(&mut self, text: &str) {
        if self.at_keyword(text) {
            self.bump_as(keyword(text).unwrap_or(SyntaxKind::BasicName));
        }
    }

    /// The index just past a `QualifiedName` written at the `n`th meaningful token, or
    /// `None` if one is not written there.
    ///
    /// `QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME` (`KerML` 8.2.3.4.1), walked
    /// exactly as `qualified_name` consumes it — including the two-token test that a
    /// `::` belongs to the name only when a NAME follows it, so `A::*` ends at `A`.
    /// A recogniser that walked it differently from the parser would accept a prefix the
    /// parser then failed to read.
    ///
    /// One pass over the tokens, not `nth_is` per step. Written when `peek_nth` filtered
    /// from the cursor and such a walk was quadratic; `peek_nth` is now a lookup in the
    /// meaningful-token index, and the single pass stays the plainer statement of it.
    pub(super) fn skip_qualified_name(&self, n: usize) -> Option<usize> {
        let mut tokens = self.meaningful_from(n);
        Some(n + self.qualified_name_length(&mut tokens)?)
    }

    /// The meaningful tokens from the `n`th on, lazily: what `peek_nth` sees, read from
    /// the meaningful-token index rather than asked for one index at a time.
    pub(super) fn meaningful_from(&self, n: usize) -> impl Iterator<Item = Token> + Clone + '_ {
        let start = self.cursor_position().saturating_add(n);
        self.meaningful_index()
            .get(start..)
            .unwrap_or(&[])
            .iter()
            .filter_map(|&i| self.tokens.get(i).copied())
    }

    /// How many tokens a `QualifiedName` at the head of `tokens` takes, consuming them,
    /// or `None` if none is written there. The rules are `skip_qualified_name`'s.
    pub(super) fn qualified_name_length(
        &self,
        tokens: &mut (impl Iterator<Item = Token> + Clone),
    ) -> Option<usize> {
        let mut length = 0;
        if tokens
            .clone()
            .next()
            .is_some_and(|token| token.kind == SyntaxKind::Dollar)
        {
            tokens.next();
            if tokens.next()?.kind != SyntaxKind::ColonColon {
                return None;
            }
            length = 2;
        }
        if !self.is_name(tokens.next()?) {
            return None;
        }
        length += 1;
        loop {
            let mut ahead = tokens.clone();
            let separator = ahead
                .next()
                .is_some_and(|t| t.kind == SyntaxKind::ColonColon);
            if !(separator && ahead.next().is_some_and(|t| self.is_name(t))) {
                return Some(length);
            }
            tokens.next();
            tokens.next();
            length += 2;
        }
    }

    /// Whether the `n`th meaningful token from here is of `kind`.
    pub(super) fn nth_is(&self, n: usize, kind: SyntaxKind) -> bool {
        self.peek_nth(n).is_some_and(|token| token.kind == kind)
    }

    /// The index just past the balanced `( ... )` opening at the `n`th token.
    pub(super) fn skip_parenthesised(&self, n: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let mut at = n;
        loop {
            let kind = self.peek_nth(at)?.kind;
            if kind == SyntaxKind::LParen {
                depth += 1;
            } else if kind == SyntaxKind::RParen {
                depth = depth.saturating_sub(1);
            }
            at += 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }

    /// The index just past the `]` matching a `[` at the `n`th token, or `None` if it is
    /// not closed before the input ends.
    pub(super) fn skip_bracketed(&self, n: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let mut at = n;
        loop {
            let kind = self.peek_nth(at)?.kind;
            if kind == SyntaxKind::LBracket {
                depth += 1;
            } else if kind == SyntaxKind::RBracket {
                depth = depth.saturating_sub(1);
            }
            at += 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }

    /// The index of the next `word` at or after the `n`th token, if one is reached before
    /// the statement ends.
    ///
    /// Bounded by the tokens that end a statement — `;` and either brace — and by the end
    /// of the input, so a truncated `if x` declines rather than scanning for ever
    /// (invariant 3). This is a token scan and not a parse, which is sound only because
    /// the words it looks for are RESERVED: `then` and `first` cannot be names
    /// (`SysML` 8.2.2.1.2), and no implemented production between them writes a `;` or a
    /// brace and then continues.
    pub(super) fn scan_for_keyword(&self, n: usize, word: &str) -> Option<usize> {
        let mut n = n;
        while !self.nth_is_keyword(n, word) {
            if self.peek_nth(n).is_none()
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::LBrace)
                || self.nth_is(n, SyntaxKind::RBrace)
            {
                return None;
            }
            n += 1;
        }
        Some(n)
    }

    /// Whether the token after the next `::` is a NAME, so the `::` is the name's.
    pub(super) fn name_follows_separator(&self) -> bool {
        self.peek_nth(1).is_some_and(|token| self.is_name(token))
    }
}
