// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The sv2 error taxonomy: every code an error leaving the CLI can carry.
//!
//! STD-002-RS §7.3 shape: a closed [`ErrorCode`] enum, and a [`Coded`] trait every
//! reportable error type implements.
//!
//! A code is added when something can produce it, never ahead of that.

use std::path::PathBuf;

/// Error codes an error leaving the CLI can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// The requested command exists in the interface but has no implementation yet.
    NotImplemented,

    /// The command line did not name a request the tool can carry out.
    Usage,

    /// The named input could not be read as UTF-8 text.
    ReadFailed,

    /// The input was read but the parser reported errors against it.
    ParseFailed,
}

impl ErrorCode {
    /// The code's stable wire name, in `SCREAMING_SNAKE_CASE`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::Usage => "USAGE",
            Self::ReadFailed => "READ_FAILED",
            Self::ParseFailed => "PARSE_FAILED",
        }
    }

    /// The process exit status this code leaves behind (STD-002-RS §3.2).
    ///
    /// One status per code, because `scripts/corpus-sweep.sh` reads the status alone
    /// and both streams are discarded: a caller that cannot tell "did not parse" from
    /// "could not be opened" would report a missing file as a grammar failure.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::NotImplemented => 2,
            Self::Usage => 3,
            Self::ReadFailed => 4,
            Self::ParseFailed => 5,
        }
    }
}

/// Implemented by every error type a binary can report.
pub trait Coded {
    /// The taxonomy code for this error.
    fn code(&self) -> ErrorCode;

    /// Whether retrying the same request unchanged could succeed.
    fn retryable(&self) -> bool {
        false
    }
}

/// Everything the `sv2` command can fail with.
///
/// Each variant carries what the reporter needs to name the thing that failed, and
/// nothing about the content of the input (§7.2 rule 3).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommandError {
    /// The command line did not name a request the tool can carry out.
    #[error("{message}")]
    Usage {
        /// What was wrong with the invocation, as one lowercase phrase.
        message: String,
    },

    /// The named command exists in the interface but has no implementation yet.
    #[error("not implemented yet; see .claude/state/state.json for what is in progress")]
    NotImplemented,

    /// The input file could not be read, or was not UTF-8.
    #[error("cannot read {path}")]
    Read {
        /// The path as it was given on the command line.
        path: PathBuf,
        /// The failure the filesystem reported.
        #[source]
        source: std::io::Error,
    },

    /// The input was read but the parser reported errors against it.
    ///
    /// The diagnostics travel with the error rather than being printed where they
    /// were found, so the one place that writes to stderr stays the one place that
    /// `--quiet` has to silence.
    #[error("{path} did not parse")]
    Parse {
        /// The path as it was given on the command line.
        path: PathBuf,
        /// What the parser could not make sense of, in source order.
        diagnostics: Vec<String>,
    },
}

impl Coded for CommandError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Usage { .. } => ErrorCode::Usage,
            Self::NotImplemented => ErrorCode::NotImplemented,
            Self::Read { .. } => ErrorCode::ReadFailed,
            Self::Parse { .. } => ErrorCode::ParseFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code, so a variant added without a wire name or a status fails here.
    const ALL: &[ErrorCode] = &[
        ErrorCode::NotImplemented,
        ErrorCode::Usage,
        ErrorCode::ReadFailed,
        ErrorCode::ParseFailed,
    ];

    #[test]
    fn wire_name_is_the_variant_in_screaming_snake_case() {
        assert_eq!(ErrorCode::NotImplemented.as_str(), "NOT_IMPLEMENTED");
        assert_eq!(ErrorCode::Usage.as_str(), "USAGE");
        assert_eq!(ErrorCode::ReadFailed.as_str(), "READ_FAILED");
        assert_eq!(ErrorCode::ParseFailed.as_str(), "PARSE_FAILED");
    }

    /// STD-002-RS §3.2: the sweep reads the status alone, so two codes that share one
    /// status are two failures a caller cannot tell apart.
    #[test]
    fn each_code_has_its_own_exit_status_and_none_of_them_means_success() {
        let mut seen: Vec<u8> = ALL.iter().map(|code| code.exit_code()).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two codes share an exit status");
        // 0 is success and 1 is a failed write (§3.2); neither is a code's to take.
        assert!(seen.iter().all(|status| *status > 1));
    }

    #[test]
    fn errors_are_not_retryable_unless_they_say_so() {
        struct Unimplemented;
        impl Coded for Unimplemented {
            fn code(&self) -> ErrorCode {
                ErrorCode::NotImplemented
            }
        }
        assert!(!Unimplemented.retryable());
    }

    #[test]
    fn a_read_failure_keeps_the_cause_it_was_given() {
        use std::error::Error as _;

        let error = CommandError::Read {
            path: PathBuf::from("model.sysml"),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        };
        assert_eq!(error.code(), ErrorCode::ReadFailed);
        assert_eq!(error.to_string(), "cannot read model.sysml");
        assert!(error.source().is_some(), "the cause was discarded");
    }
}
