// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! What a tool says about a span of text.
//!
//! Defined here because `sv2-syntax` is the lowest crate that already owns
//! [`TextRange`], and every crate above it needs the type. One vocabulary, not one per
//! layer: codes are namespaced by the crate that raises them (`PARSE-*` here, `HIR-*`,
//! `RES-*` and `ID-*` above), and no crate defines its own enum for user-facing output.
//!
//! This is NOT `sv2-cli`'s `ErrorCode`'s job, and the two must not be merged.
//! That enum is a taxonomy of how the *process* failed — the file could
//! not be opened, the command line was not understood — and it maps one-to-one onto
//! exit statuses (STD-002-RS §3.2). A `Diagnostic` is about the *model*: it has a
//! position in a file, and a hundred of them still leave the process exit status at
//! "this file did not parse". A caller that could not tell those apart would report a
//! mistyped path as a grammar failure.
//!
//! A diagnostic carries a range so that it can be pointed at. ADR-0002 requires an
//! element with an unresolved reference to be *decorated* rather than filtered out, and
//! a decoration needs somewhere to go; an editor underlining a token needs the same
//! thing. Neither is expressible in a sentence of prose.

use text_size::TextRange;

/// How much a diagnostic matters.
///
/// [`Severity::Error`] is raised for text the language does not admit, and
/// [`Severity::Info`] for text admitted only by a recorded deviation from the
/// specification (`PARSE-DEVIATION`, ADR-0022), which the parser keeps apart from its
/// errors. `only_errors_and_deviations_are_raised` in the parser's tests holds that down.
/// [`Severity::Warning`] has no producer yet; ADR-0002 decorates rather than filters, and
/// ADR-0016 names an informational diagnostic of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// The text is not what the language admits. Gates a write; never gates a read.
    Error,
    /// Admitted, but probably not meant.
    Warning,
    /// Something the tool did that the author may want to know about.
    Info,
}

impl Severity {
    /// The severity's stable wire name, lowercase.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// Every code a diagnostic raised by this crate can carry.
///
/// Closed, and `#[non_exhaustive]` so adding one is not a breaking change downstream
/// (STD-002-RS §2 rule 3). A code is added when something can produce it, never ahead
/// of that — the same rule `sv2-cli`'s `ErrorCode` follows. It cannot be linked from
/// here: `sv2-cli` is four layers up, and the dependency only points downward.
///
/// The `PARSE-` prefix is the namespace of the crate that raises it. A code from
/// `sv2-hir` will read `HIR-`, and the two will never collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// A production needed something here and found something else.
    Expected,
    /// Text that no production admits at this point.
    Unexpected,
    /// A `/* ... */` that the end of the file arrived in the middle of.
    UnterminatedComment,
    /// Input nested deeper than the parser will recurse (invariant 3).
    TooDeeplyNested,
    /// Text the parser admits only because of a recorded deviation from the
    /// specification's BNF, whose register entry the message names (ADR-0022).
    Deviation,
}

impl DiagnosticCode {
    /// The code's stable wire name, `SCREAMING-KEBAB-CASE` behind its namespace.
    ///
    /// Stable in the sense that matters: it is what a suppression comment, a test, or
    /// a downstream consumer names, so it may not be respelled once published.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Expected => "PARSE-EXPECTED",
            Self::Unexpected => "PARSE-UNEXPECTED",
            Self::UnterminatedComment => "PARSE-UNTERMINATED-COMMENT",
            Self::TooDeeplyNested => "PARSE-TOO-DEEPLY-NESTED",
            Self::Deviation => "PARSE-DEVIATION",
        }
    }

    /// The severity a diagnostic with this code carries.
    ///
    /// A property of the code rather than of the site, so that two raisers of the same
    /// code cannot disagree about how much it matters.
    #[must_use]
    pub fn severity(self) -> Severity {
        match self {
            Self::Expected
            | Self::Unexpected
            | Self::UnterminatedComment
            | Self::TooDeeplyNested => Severity::Error,
            Self::Deviation => Severity::Info,
        }
    }
}

/// One thing a tool has to say about one span of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    range: TextRange,
    message: String,
}

impl Diagnostic {
    /// A diagnostic at `range`, carrying `code` and a rendered `message`.
    ///
    /// The severity comes from the code, so a caller cannot raise the same code at two
    /// different severities.
    #[must_use]
    pub fn new(code: DiagnosticCode, range: TextRange, message: String) -> Self {
        Self {
            code,
            range,
            message,
        }
    }

    /// What kind of thing this is.
    #[must_use]
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// How much it matters.
    #[must_use]
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// The half-open byte range it is about.
    ///
    /// May be empty, and an empty range is not a defect: "expected `}`, found end of
    /// file" is about a position rather than about any bytes, and it still has to be
    /// pointed at.
    #[must_use]
    pub fn range(&self) -> TextRange {
        self.range
    }

    /// The rendered message, for a person.
    ///
    /// One lowercase phrase, no trailing period, and it never contains the position —
    /// the range is the position, and a caller renders it the way its output wants.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code, so a variant added without a wire name or a severity fails here.
    const ALL: &[DiagnosticCode] = &[
        DiagnosticCode::Expected,
        DiagnosticCode::Unexpected,
        DiagnosticCode::UnterminatedComment,
        DiagnosticCode::TooDeeplyNested,
    ];

    #[test]
    fn every_code_is_namespaced_to_the_crate_that_raises_it() {
        for code in ALL {
            assert!(
                code.as_str().starts_with("PARSE-"),
                "{code:?} is raised by sv2-syntax and must say so"
            );
        }
    }

    #[test]
    fn no_two_codes_share_a_wire_name() {
        let mut seen: Vec<&str> = ALL.iter().map(|code| code.as_str()).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two codes share a wire name");
    }

    #[test]
    fn severity_comes_from_the_code_and_not_from_the_site() {
        let range = TextRange::new(0.into(), 1.into());
        let one = Diagnostic::new(DiagnosticCode::Expected, range, "a".to_owned());
        let other = Diagnostic::new(DiagnosticCode::Expected, range, "b".to_owned());
        assert_eq!(one.severity(), other.severity());
        assert_eq!(one.severity(), Severity::Error);
    }

    #[test]
    fn an_empty_range_is_representable() {
        // "expected `}`, found end of file" is about a position, not about any bytes.
        let end = TextRange::empty(12.into());
        let diagnostic = Diagnostic::new(DiagnosticCode::Expected, end, "x".to_owned());
        assert!(diagnostic.range().is_empty());
        assert_eq!(diagnostic.range().start(), 12.into());
    }
}
