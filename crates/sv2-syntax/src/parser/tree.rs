// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Building the tree: opening and closing nodes, consuming tokens, and attaching trivia,
//! including the retroactive nodes a production opens once it knows what it read.

use rowan::Language as _;

use crate::diagnostic::DiagnosticCode;
use crate::generated::kinds::SyntaxKind;
use crate::language::Sv2Language;
use crate::lexer::{Token, is_unterminated_comment};
use crate::parser::Parser;

impl Parser<'_> {
    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(Sv2Language::kind_to_raw(kind));
    }

    pub(super) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Attach pending trivia to the tree. Never skipped — losslessness depends on it.
    ///
    /// A `REGULAR_COMMENT` reaching here as trivia is in no position a `Comment` may
    /// stand, so it is reported, and still kept in the tree. `KerML` 8.2.2.2 makes `/* ...
    /// */` a token, not a note: the notes are `//* ... */` and `// ...`, which the Pilot
    /// hides with whitespace (KerMLExpressions.xtext:29) and this does too.
    pub(super) fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos).copied() {
            if !self.skippable(token.kind) {
                break;
            }
            if token.kind == SyntaxKind::RegularComment {
                self.emit(
                    DiagnosticCode::Unexpected,
                    Self::range_of(token),
                    "a `/* ... */` comment is an element, and no element may stand here; \
                     write `//* ... */` for a note"
                        .to_owned(),
                );
            }
            self.push(token, token.kind);
        }
    }

    /// Every token enters the tree here exactly once, which is why the
    /// unterminated-comment diagnostic is raised here: a regular comment may arrive
    /// as trivia or as an annotation's body, and both must report it.
    pub(super) fn push(&mut self, token: Token, kind: SyntaxKind) {
        if is_unterminated_comment(token.kind, self.text_of(token)) {
            self.emit(
                DiagnosticCode::UnterminatedComment,
                Self::range_of(token),
                "comment is never closed: expected `*/`".to_owned(),
            );
        }
        self.builder
            .token(Sv2Language::kind_to_raw(kind), self.text_of(token));
        self.pos += 1;
    }

    /// Consume the next non-trivia token, with the trivia before it.
    pub(super) fn bump(&mut self) {
        if let Some(token) = self.peek() {
            self.bump_as(token.kind);
        }
    }

    /// Consume the next non-trivia token, tagged as `kind` rather than as it lexed.
    ///
    /// Keywords reach the parser as `BasicName`; the production that recognises one
    /// says so here, and the tree records the keyword the token set names.
    pub(super) fn bump_as(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos).copied() {
            crate::counter::consumed();
            self.push(token, kind);
        }
    }

    // -- retroactive nodes -------------------------------------------------------

    /// Open `kind` retroactively at `start`, over what is already in the tree.
    pub(super) fn start_node_at(&mut self, start: rowan::Checkpoint, kind: SyntaxKind) {
        self.builder
            .start_node_at(start, Sv2Language::kind_to_raw(kind));
    }

    /// Wrap what is already in the tree at `start` in `kinds`, outermost first.
    ///
    /// rowan's parent stack finishes in reverse, so the first kind opened is the
    /// outermost one and `&[ArgumentMember, Argument, ArgumentValue]` nests in the
    /// order the clause writes them.
    pub(super) fn wrap_at(&mut self, start: rowan::Checkpoint, kinds: &[SyntaxKind]) {
        for kind in kinds {
            self.start_node_at(start, *kind);
        }
        for _ in kinds {
            self.finish_node();
        }
    }
}
