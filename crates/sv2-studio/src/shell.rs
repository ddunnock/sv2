// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Starting the studio: which workspace, then the window.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::commands;
use crate::workspace::WorkspaceRoot;

/// Exit status for every failure to start: a bad argument, or a window that would not
/// open. Nothing reads the status but a person, so one is enough.
const FAILED: u8 = 1;

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
///
/// On success this returns only when the last window closes.
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    _stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> ExitCode {
    let root = workspace_root(args, std::env::current_dir).and_then(|r| directory(&r));
    let root = match root {
        Ok(root) => root,
        // §8.2: a message, not the command's product, so stdout stays empty.
        Err(message) => return fail(stderr, &message),
    };
    let started = tauri::Builder::default()
        .manage(WorkspaceRoot(root))
        .invoke_handler(tauri::generate_handler![
            commands::workspace,
            commands::file_text,
            commands::views,
            commands::element_detail,
            commands::view_layout,
        ])
        .run(context());
    match started {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(stderr, &format!("the window did not open: {error}")),
    }
}

/// `tauri.conf.json` and `capabilities/`, compiled in.
#[expect(
    clippy::exit,
    reason = "the macro's generated code exits on its own failures; nothing here calls it"
)]
fn context() -> tauri::Context {
    tauri::generate_context!()
}

/// The workspace to open: the one argument after the program name, or else the
/// current directory. `cwd` is asked only when there is no argument.
fn workspace_root(
    args: impl IntoIterator<Item = OsString>,
    cwd: impl FnOnce() -> std::io::Result<PathBuf>,
) -> Result<PathBuf, String> {
    let mut args = args.into_iter().skip(1);
    match (args.next(), args.next()) {
        (Some(root), None) => Ok(PathBuf::from(root)),
        (None, _) => cwd().map_err(|e| format!("no workspace given, and {e}")),
        (Some(_), Some(_)) => Err("usage: sv2-studio [WORKSPACE]".to_owned()),
    }
}

/// `root`, made absolute, if it is a directory. Absolute so the root row's name is the
/// directory's own even when it was given as `.`.
fn directory(root: &Path) -> Result<PathBuf, String> {
    match root.canonicalize() {
        Ok(path) if path.is_dir() => Ok(path),
        Ok(_) => Err(format!("{} is not a directory", root.display())),
        Err(e) => Err(format!("cannot open {}: {e}", root.display())),
    }
}

/// Write `message` to `stderr` and exit with `FAILED`.
fn fail(stderr: &mut impl Write, message: &str) -> ExitCode {
    // If even this write fails there is nowhere left to say so; the status still does.
    let _unreported = writeln!(stderr, "sv2-studio: {message}");
    ExitCode::from(FAILED)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn no_cwd() -> std::io::Result<PathBuf> {
        panic!("the current directory was asked for although an argument was given")
    }

    #[test]
    fn the_one_argument_is_the_workspace() {
        assert_eq!(
            workspace_root(args(&["sv2-studio", "models/thermal"]), no_cwd),
            Ok(PathBuf::from("models/thermal"))
        );
    }

    #[test]
    fn with_no_argument_the_workspace_is_the_current_directory() {
        assert_eq!(
            workspace_root(args(&["sv2-studio"]), || Ok(PathBuf::from("/here"))),
            Ok(PathBuf::from("/here"))
        );
    }

    #[test]
    fn an_unreadable_current_directory_is_reported() {
        let result = workspace_root(args(&["sv2-studio"]), || {
            Err(std::io::Error::other("it was removed"))
        });
        assert_eq!(
            result,
            Err("no workspace given, and it was removed".to_owned())
        );
    }

    #[test]
    fn two_arguments_are_refused_with_the_usage() {
        assert_eq!(
            workspace_root(args(&["sv2-studio", "a", "b"]), no_cwd),
            Err("usage: sv2-studio [WORKSPACE]".to_owned())
        );
    }

    #[test]
    fn a_workspace_that_is_not_a_directory_fails_before_any_window() {
        let file = std::env::temp_dir().join(format!("sv2-studio-file-{}", std::process::id()));
        std::fs::write(&file, "").unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [OsString::from("sv2-studio"), file.clone().into_os_string()],
            &mut stdout,
            &mut stderr,
        );
        std::fs::remove_file(&file).unwrap();

        assert_eq!(code, ExitCode::from(FAILED));
        assert!(
            stdout.is_empty(),
            "stdout carries the product and there is none"
        );
        let said = String::from_utf8(stderr).unwrap();
        assert!(said.contains("is not a directory"), "{said}");
    }

    #[test]
    fn a_missing_workspace_fails_before_any_window() {
        let mut stderr = Vec::new();
        let code = run(
            args(&["sv2-studio", "/no/such/sv2/workspace"]),
            &mut Vec::new(),
            &mut stderr,
        );
        assert_eq!(code, ExitCode::from(FAILED));
        let said = String::from_utf8(stderr).unwrap();
        assert!(
            said.contains("cannot open /no/such/sv2/workspace"),
            "{said}"
        );
    }
}
