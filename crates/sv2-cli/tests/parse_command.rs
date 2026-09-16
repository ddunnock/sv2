// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! What `sv2 parse` does with a file on disk.
//!
//! The argument shapes are held down in `src/cli.rs`, where no filesystem is needed.
//! These are the cases that need one, and they call `sv2_cli::run` directly rather
//! than spawning the binary: the contract is the exit status and the two streams
//! (STD-002-RS §3.4), and all three are reachable without a subprocess.
//!
//! The one thing `scripts/corpus-sweep.sh` relies on is checked here in the form it
//! uses it: `parse --quiet <file>`, status alone, both streams discarded.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sv2_cli::ErrorCode;

/// A file under the test target's own temporary directory, named for its case.
///
/// `CARGO_TARGET_TMPDIR` is cleaned by cargo and is per-target, so two cases cannot
/// collide and nothing is left in the source tree.
///
/// §6.3 allows tests to unwrap, and `allow-unwrap-in-tests` covers a `#[test]` body;
/// it does not reach a helper called from one, so the exception is stated here.
#[expect(
    clippy::unwrap_used,
    reason = "a fixture that cannot be written leaves no test to run, and the panic names which"
)]
fn model(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("parse_command");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Run the command, returning the status and both streams as text.
#[expect(
    clippy::unwrap_used,
    reason = "the command writes text to both streams; bytes that are not are a defect"
)]
fn invoke(args: &[&str]) -> (ExitCode, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = sv2_cli::run(args.iter().map(OsString::from), &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn parse(path: &Path) -> (ExitCode, String, String) {
    invoke(&["sv2", "parse", &path.to_string_lossy()])
}

fn exit(code: ErrorCode) -> ExitCode {
    ExitCode::from(code.exit_code())
}

/// `SysML` 8.2.2.5.1: `PackageBody = ';' | '{' PackageBodyElement* '}'`, so a package
/// whose body is a semicolon is a package, and the parser has nothing to report.
#[test]
fn a_file_the_parser_accepts_exits_zero_and_says_nothing() {
    let path = model("accepted.sysml", b"package Vehicle;");
    let (code, out, err) = parse(&path);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(
        out.is_empty(),
        "stdout carries the product, and there is none"
    );
    assert!(err.is_empty(), "nothing to report: {err}");
}

/// The negative case for the same clause: the declaration is there, the body is not.
#[test]
fn a_file_the_parser_rejects_exits_with_parse_failed_and_reports_why() {
    let path = model("rejected.sysml", b"package Vehicle");
    let (code, out, err) = parse(&path);
    assert_eq!(code, exit(ErrorCode::ParseFailed));
    assert!(out.is_empty(), "a diagnostic is not the command's product");
    assert!(err.contains(ErrorCode::ParseFailed.as_str()));
    assert!(err.contains("rejected.sysml"), "the report names the file");
    // The parser's own diagnostics reach the caller, not just the fact of failure.
    assert!(
        err.lines().count() > 1,
        "only the summary was written: {err}"
    );
}

/// The sweep's exact invocation: the status is the answer, and nothing is printed.
#[test]
fn quiet_silences_the_report_and_leaves_the_status_alone() {
    let path = model("quiet.sysml", b"package Vehicle");
    let loud = parse(&path);
    let (code, out, err) = invoke(&["sv2", "parse", "--quiet", &path.to_string_lossy()]);
    assert_eq!(
        code, loud.0,
        "--quiet changed the answer, not just the noise"
    );
    assert_eq!(code, exit(ErrorCode::ParseFailed));
    assert!(out.is_empty());
    assert!(err.is_empty(), "--quiet still wrote: {err}");
}

/// A file that is not there is not a file that did not parse. The sweep discards both
/// streams, so a shared exit status would report a mistyped path as a grammar failure.
#[test]
fn a_missing_file_is_read_failed_and_not_parse_failed() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("parse_command/no-such-file.sysml");
    let (code, out, err) = parse(&path);
    assert_eq!(code, exit(ErrorCode::ReadFailed));
    assert_ne!(code, exit(ErrorCode::ParseFailed));
    assert!(out.is_empty());
    assert!(err.contains("no-such-file.sysml"));
    assert!(err.contains("caused by:"), "the cause was dropped: {err}");
}

/// `KerML` 8.2.2.1: source text is Unicode. Bytes that are not are unreadable input,
/// not a model the parser declined — and the parser never sees them.
#[test]
fn a_file_that_is_not_utf8_is_read_failed() {
    let path = model("not-utf8.sysml", b"package \xff\xfe;");
    let (code, out, err) = parse(&path);
    assert_eq!(code, exit(ErrorCode::ReadFailed));
    assert!(out.is_empty());
    assert!(err.contains("not-utf8.sysml"));
}

/// Losslessness is a property of the tree, and `parse` must not quietly rewrite the
/// file it was pointed at. Nothing in the command opens the file for writing; this is
/// what would notice if that changed.
#[test]
fn parsing_does_not_touch_the_file() {
    let source: &[u8] = b"package  Vehicle ;\r\n// a comment with trailing space \n";
    let path = model("untouched.sysml", source);
    let (code, _, _) = parse(&path);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(std::fs::read(&path).unwrap(), source);
}
