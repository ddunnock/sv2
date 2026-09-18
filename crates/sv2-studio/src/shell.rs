// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Starting the studio, and reporting that it cannot yet.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

/// Exit status when a stream write fails, leaving nothing reportable.
const WRITE_FAILED: u8 = 1;
/// Exit status for a request this binary does not implement yet.
const NOT_IMPLEMENTED: u8 = 2;

/// Run the studio, writing only to the given streams.
///
/// Takes its arguments and its output streams as parameters for the reason `sv2` does
/// (STD-002-RS §2.2): all of the behaviour is in the library target, so a test passes a
/// `Vec<u8>` and asserts on what was written rather than spawning a process.
///
/// Exit statuses here are **not** `sv2`'s taxonomy. That one maps one status per
/// `ErrorCode` because the corpus sweep reads the status alone (§3.2); a window that
/// failed to open is not that kind of answer. Sharing the enum would tie a GUI's
/// failures to a batch tool's contract, which is the confusion ADR-0018 separates.
pub fn run(
    _args: impl IntoIterator<Item = OsString>,
    _stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> ExitCode {
    // §8.2: a message, not the command's product, so stdout stays empty.
    let said = writeln!(
        stderr,
        "sv2-studio: NOT_IMPLEMENTED: the studio does not start yet; \
         see .claude/state/state.json for what is in progress"
    );
    if said.is_err() {
        return ExitCode::from(WRITE_FAILED);
    }
    ExitCode::from(NOT_IMPLEMENTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stub_says_so_on_stderr_and_leaves_stdout_empty() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(Vec::<OsString>::new(), &mut stdout, &mut stderr);

        assert_eq!(code, ExitCode::from(NOT_IMPLEMENTED));
        assert!(
            stdout.is_empty(),
            "stdout carries the product and there is none"
        );
        let said = String::from_utf8(stderr).unwrap();
        assert!(said.contains("NOT_IMPLEMENTED"), "{said}");
        assert!(said.contains(".claude/state/state.json"), "{said}");
    }
}
