// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Dispatch for the `sv2` command.
//!
//! Two requests are implemented: `--version`, and `parse`, which reads one file and
//! says whether the parser accepts it. Everything else reports that it is not
//! implemented and exits 2.
//!
//! `scripts/corpus-sweep.sh` reads the exit status alone and discards both streams
//! (STD-002-RS §3.4), so the status is the contract and the text beside it is for a
//! person in a terminal.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::error::{Coded as _, CommandError};

/// Exit status when a stream write fails, leaving nothing reportable (§3.2).
const WRITE_FAILED: u8 = 1;

/// Whether a failure about the input is spoken aloud or left to the exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reporting {
    /// Write the error, its cause, and the parser's diagnostics to stderr.
    Loud,
    /// Say nothing about the file: `--quiet`, and what the corpus sweep passes.
    Silent,
}

/// What a command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Request {
    /// Print the version.
    Version,
    /// Parse one file and report whether the parser accepts it.
    Parse {
        /// The file to read, as it was given.
        path: PathBuf,
        /// What to say when it does not parse.
        reporting: Reporting,
    },
}

/// Why the command did not succeed.
enum Failure {
    /// A write failed, so there is no way left to report anything.
    Write,
    /// The command failed and has already said so on stderr.
    Reported(CommandError),
}

/// Runs `sv2` with `args` (program name first), writing only to the given streams.
///
/// `--version` prints the version to `stdout` and succeeds. `parse <file>` exits 0
/// when the parser reports nothing against the file. Every other invocation exits
/// with the status of its error code ([`crate::ErrorCode::exit_code`]), or 1 when a
/// write failed.
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> ExitCode {
    match attempt(args, stdout, stderr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Write) => ExitCode::from(WRITE_FAILED),
        Err(Failure::Reported(error)) => ExitCode::from(error.code().exit_code()),
    }
}

/// Carry out the request, reporting any failure on `stderr` before returning it.
fn attempt(
    args: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), Failure> {
    let request = request(args);
    // Loud for every request that did not parse, which is what keeps `--quiet` from
    // silencing a complaint about the command line itself.
    let reporting = match request {
        Ok(Request::Parse { reporting, .. }) => reporting,
        _ => Reporting::Loud,
    };
    let outcome = match request {
        Ok(Request::Version) => {
            let version = writeln!(stdout, "sv2 {}", env!("CARGO_PKG_VERSION"));
            return version.map_err(|_| Failure::Write);
        }
        Ok(Request::Parse { path, .. }) => parse_file(&path),
        Err(error) => Err(error),
    };
    match outcome {
        Ok(()) => Ok(()),
        Err(error) => {
            report(&error, reporting, stderr).map_err(|_| Failure::Write)?;
            Err(Failure::Reported(error))
        }
    }
}

/// The request `args` names, with the program name dropped.
fn request(args: impl IntoIterator<Item = OsString>) -> Result<Request, CommandError> {
    let args: Vec<OsString> = args.into_iter().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") {
        return Ok(Request::Version);
    }
    let mut args = args.into_iter();
    match args.next() {
        Some(command) if command == "parse" => parse_request(args),
        // Including no command at all: `sv2` on its own names nothing this tool can do.
        _ => Err(CommandError::NotImplemented),
    }
}

/// `parse [--quiet] [--] <file>` — the arguments after the command word.
fn parse_request(args: impl Iterator<Item = OsString>) -> Result<Request, CommandError> {
    let mut reporting = Reporting::Loud;
    let mut path: Option<PathBuf> = None;
    let mut positional_only = false;
    for arg in args {
        if !positional_only && arg == "--" {
            positional_only = true;
        } else if !positional_only && arg == "--quiet" {
            reporting = Reporting::Silent;
        } else if !positional_only && is_option(&arg) {
            let name = arg.to_string_lossy();
            return Err(usage(format!("parse: unknown option {name}")));
        } else if path.replace(PathBuf::from(arg)).is_some() {
            return Err(usage("parse: takes one file".to_owned()));
        }
    }
    path.map(|path| Request::Parse { path, reporting })
        .ok_or_else(|| usage("parse: needs a file to parse".to_owned()))
}

/// Whether `arg` reads as an option rather than a path.
///
/// Byte-wise, because a path is not required to be UTF-8 and a lossy conversion here
/// would turn an unreadable name into a different unreadable name.
fn is_option(arg: &OsStr) -> bool {
    arg.as_encoded_bytes().first() == Some(&b'-')
}

/// A usage error carrying `message`.
fn usage(message: String) -> CommandError {
    CommandError::Usage { message }
}

/// Read `path` and parse it, failing when the parser has anything to report.
///
/// Acceptance is "the parser reported nothing", never "a tree came back": `parse`
/// always returns a tree, because the tree keeps every byte whether or not the text is
/// well formed (ADR-0004). The diagnostics are what say whether it is a model.
fn parse_file(path: &Path) -> Result<(), CommandError> {
    // §3.3: the file is opened after the argument naming it was understood, not before.
    let source = std::fs::read_to_string(path).map_err(|source| CommandError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed = sv2_syntax::parse(&source);
    if parsed.errors().is_empty() {
        return Ok(());
    }
    Err(CommandError::Parse {
        path: path.to_path_buf(),
        diagnostics: parsed.errors().to_vec(),
    })
}

/// Write the error to `stderr`: one line naming its code, then what it is made of.
///
/// §8.2: this is a message, not the command's product, so it goes to stderr and
/// stdout stays empty.
fn report(
    error: &CommandError,
    reporting: Reporting,
    stderr: &mut impl Write,
) -> std::io::Result<()> {
    if reporting == Reporting::Silent {
        return Ok(());
    }
    writeln!(stderr, "sv2: {}: {error}", error.code().as_str())?;
    let mut cause = std::error::Error::source(error);
    while let Some(current) = cause {
        writeln!(stderr, "  caused by: {current}")?;
        cause = current.source();
    }
    if let CommandError::Parse { diagnostics, .. } = error {
        for diagnostic in diagnostics {
            writeln!(stderr, "  {diagnostic}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorCode;

    fn invoke(args: &[&str]) -> (ExitCode, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(args.iter().map(OsString::from), &mut stdout, &mut stderr);
        let out = String::from_utf8(stdout).unwrap();
        let err = String::from_utf8(stderr).unwrap();
        (code, out, err)
    }

    fn exit(code: ErrorCode) -> ExitCode {
        ExitCode::from(code.exit_code())
    }

    #[test]
    fn version_prints_to_stdout_and_succeeds() {
        let (code, out, err) = invoke(&["sv2", "--version"]);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.starts_with("sv2 "));
        assert!(err.is_empty());
    }

    #[test]
    fn unimplemented_command_exits_two_and_points_at_state() {
        let (code, out, err) = invoke(&["sv2", "resolve", "model.sysml"]);
        assert_eq!(code, exit(ErrorCode::NotImplemented));
        assert!(out.is_empty());
        assert!(err.contains(".claude/state/state.json"));
        assert!(err.contains(ErrorCode::NotImplemented.as_str()));
    }

    #[test]
    fn no_command_at_all_is_not_implemented() {
        let (code, out, _) = invoke(&["sv2"]);
        assert_eq!(code, exit(ErrorCode::NotImplemented));
        assert!(out.is_empty());
    }

    #[test]
    fn parse_takes_one_file() {
        assert_eq!(
            request(["sv2", "parse", "a.sysml"].map(OsString::from)).unwrap(),
            Request::Parse {
                path: PathBuf::from("a.sysml"),
                reporting: Reporting::Loud,
            }
        );
        assert_eq!(
            request(["sv2", "parse", "--quiet", "a.sysml"].map(OsString::from)).unwrap(),
            Request::Parse {
                path: PathBuf::from("a.sysml"),
                reporting: Reporting::Silent,
            }
        );
    }

    /// `--` is what lets a file whose name begins with `-` be named at all.
    #[test]
    fn a_path_after_the_separator_is_a_path_however_it_starts() {
        assert_eq!(
            request(["sv2", "parse", "--", "--quiet"].map(OsString::from)).unwrap(),
            Request::Parse {
                path: PathBuf::from("--quiet"),
                reporting: Reporting::Loud,
            }
        );
    }

    #[test]
    fn parse_without_a_file_is_a_usage_error() {
        let (code, out, err) = invoke(&["sv2", "parse", "--quiet"]);
        assert_eq!(code, exit(ErrorCode::Usage));
        assert!(out.is_empty());
        assert!(err.contains(ErrorCode::Usage.as_str()));
        // --quiet is about the file; it cannot silence a complaint about the invocation.
        assert!(err.contains("needs a file"));
    }

    #[test]
    fn parse_with_two_files_is_a_usage_error() {
        let (code, _, err) = invoke(&["sv2", "parse", "a.sysml", "b.sysml"]);
        assert_eq!(code, exit(ErrorCode::Usage));
        assert!(err.contains("one file"));
    }

    #[test]
    fn an_unknown_option_is_a_usage_error_and_names_the_option() {
        let (code, _, err) = invoke(&["sv2", "parse", "--verbose", "a.sysml"]);
        assert_eq!(code, exit(ErrorCode::Usage));
        assert!(err.contains("--verbose"));
    }
}
