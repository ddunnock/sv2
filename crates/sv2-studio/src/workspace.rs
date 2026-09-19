// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The open workspace: its model files, and reading one.
//!
//! READ-ONLY. ADR-0020 makes opening a workspace allocate identities, which
//! writes every file; this crate does not do that yet, and writes nothing. It
//! lists `.sysml` and `.kerml` files, parses each with `sv2-syntax` to count
//! its diagnostics, and reads a file's text on request.
//!
//! PATHS CROSS IN ONE FORM (STD-004-TS §4.2, `contract/file.ts`): relative to
//! the root, `/`-separated. The conversion happens here and nowhere else, and
//! a requested path is refused unless every component is a plain name, so no
//! request can read outside the root however it is spelled.

use std::fs;
use std::path::{Component, Path, PathBuf};

use sv2_syntax::{Language as Grammar, Severity};

use crate::wire::{Answer, DiagnosticCounts, FileText, Language, Workspace, WorkspaceFile};

/// The workspace root the studio was opened on.
#[derive(Debug, Clone)]
pub(crate) struct WorkspaceRoot(pub(crate) PathBuf);

/// Directories never walked: tool state, build output, and hidden directories.
const SKIPPED: [&str; 2] = ["target", "node_modules"];

impl WorkspaceRoot {
    /// Every model file under the root, sorted by path, with diagnostic counts.
    pub(crate) fn scan(&self) -> Workspace {
        let mut files = Vec::new();
        self.walk(&self.0, &mut files);
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let name = self.0.file_name().map_or_else(
            || self.0.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        Workspace { name, files }
    }

    fn walk(&self, dir: &Path, files: &mut Vec<WorkspaceFile>) {
        // An unreadable directory is left out rather than failing the whole
        // workspace; the tree shows what could be read.
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let walked = !name.starts_with('.') && !SKIPPED.contains(&name.as_str());
            match (path.is_dir(), walked) {
                (true, true) => self.walk(&path, files),
                (true, false) => {}
                (false, _) => files.extend(self.model_file(&path)),
            }
        }
    }

    /// A `WorkspaceFile` for `path` if it is a readable model file.
    fn model_file(&self, path: &Path) -> Option<WorkspaceFile> {
        let language = language_of(path)?;
        let relative = wire_path(path.strip_prefix(&self.0).ok()?)?;
        let text = fs::read_to_string(path).ok()?;
        Some(WorkspaceFile {
            path: relative,
            language,
            diagnostics: count(&text, language),
        })
    }

    /// The text of the model file at the workspace path `requested`.
    pub(crate) fn read(&self, requested: &str) -> Answer<FileText> {
        let Some(relative) = safe_relative(requested) else {
            return Answer::not_found();
        };
        let path = self.0.join(relative);
        if language_of(&path).is_none() {
            return Answer::not_found();
        }
        match fs::read(&path) {
            Err(_) => Answer::not_found(),
            // Not UTF-8: refused rather than decoded lossily, because the text
            // is the model and a replacement character would change it (ADR-0004).
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => Answer::Ready {
                    data: FileText {
                        path: requested.to_owned(),
                        text,
                    },
                },
                Err(_) => Answer::not_implemented("reading a file that is not UTF-8"),
            },
        }
    }
}

/// The grammar for a file, by its extension (ADR-0014).
fn language_of(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "sysml" => Some(Language::Sysml),
        "kerml" => Some(Language::Kerml),
        _ => None,
    }
}

/// `path`'s components joined with `/`, or None if any is not a plain UTF-8 name.
fn wire_path(path: &Path) -> Option<String> {
    let parts: Option<Vec<&str>> = path
        .components()
        .map(|c| match c {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let parts = parts?;
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// `requested` as a relative path of plain components only, or None.
fn safe_relative(requested: &str) -> Option<PathBuf> {
    let mut path = PathBuf::new();
    for part in requested.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
            return None;
        }
        path.push(part);
    }
    // Belt and braces: whatever `push` made of the parts must still be plain.
    path.components()
        .all(|c| matches!(c, Component::Normal(_)))
        .then_some(path)
}

/// The file's diagnostics, counted by severity.
fn count(text: &str, language: Language) -> DiagnosticCounts {
    let grammar = match language {
        Language::Sysml => Grammar::SysMl,
        Language::Kerml => Grammar::KerMl,
    };
    let mut counts = DiagnosticCounts::default();
    for diagnostic in sv2_syntax::parse(text, grammar).errors() {
        let slot = match diagnostic.severity() {
            Severity::Error => &mut counts.error,
            Severity::Warning => &mut counts.warning,
            // `Severity` is non-exhaustive. A severity this crate does not know
            // is still counted, as the least alarming kind, rather than dropped:
            // ADR-0002 decorates by diagnostic state, never filters on it.
            _ => &mut counts.info,
        };
        *slot = slot.saturating_add(1);
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    /// A fresh directory under the system temp dir, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("sv2-studio-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, relative: &str, text: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _removed = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn scan_lists_model_files_by_path_with_their_language() {
        let dir = Scratch::new();
        dir.write("model/B.sysml", "package B;\n");
        dir.write("library/A.kerml", "package A;\n");
        dir.write("notes.txt", "not a model file");
        let workspace = WorkspaceRoot(dir.0.clone()).scan();
        let listed: Vec<(&str, Language)> = workspace
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.language))
            .collect();
        assert_eq!(
            listed,
            [
                ("library/A.kerml", Language::Kerml),
                ("model/B.sysml", Language::Sysml)
            ]
        );
    }

    #[test]
    fn scan_skips_hidden_and_tool_directories() {
        let dir = Scratch::new();
        dir.write(".git/x.sysml", "package X;\n");
        dir.write("target/y.sysml", "package Y;\n");
        dir.write("node_modules/z.sysml", "package Z;\n");
        dir.write("m.sysml", "package M;\n");
        let paths: Vec<String> = WorkspaceRoot(dir.0.clone())
            .scan()
            .files
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(paths, ["m.sysml"]);
    }

    #[test]
    fn scan_counts_the_parser_diagnostics() {
        let dir = Scratch::new();
        dir.write("clean.sysml", "package P;\n");
        dir.write("broken.sysml", "package {\n");
        let workspace = WorkspaceRoot(dir.0.clone()).scan();
        let counts = |path: &str| {
            workspace
                .files
                .iter()
                .find(|f| f.path == path)
                .map(|f| f.diagnostics)
                .unwrap()
        };
        assert_eq!(counts("clean.sysml"), DiagnosticCounts::default());
        assert!(counts("broken.sysml").error > 0);
    }

    #[test]
    fn read_returns_the_text_exactly() {
        let dir = Scratch::new();
        dir.write("model/A.sysml", "package A {\n  // é\n}\n");
        assert_eq!(
            WorkspaceRoot(dir.0.clone()).read("model/A.sysml"),
            Answer::Ready {
                data: FileText {
                    path: "model/A.sysml".to_owned(),
                    text: "package A {\n  // é\n}\n".to_owned()
                }
            }
        );
    }

    #[test]
    fn read_refuses_every_path_that_could_leave_the_root() {
        let dir = Scratch::new();
        dir.write("model/A.sysml", "package A;\n");
        let root = WorkspaceRoot(dir.0.join("model"));
        for requested in [
            "../model/A.sysml",
            "/etc/passwd.sysml",
            "./A.sysml",
            "a//A.sysml",
            "..\\A.sysml",
            "",
        ] {
            assert_eq!(root.read(requested), Answer::not_found(), "{requested}");
        }
    }

    #[test]
    fn read_refuses_a_file_that_is_not_a_model_file() {
        let dir = Scratch::new();
        dir.write("secrets.txt", "no");
        assert_eq!(
            WorkspaceRoot(dir.0.clone()).read("secrets.txt"),
            Answer::not_found()
        );
    }

    #[test]
    fn read_does_not_decode_a_non_utf8_file_lossily() {
        let dir = Scratch::new();
        fs::write(dir.0.join("bad.sysml"), [0x70, 0xff, 0x0a]).unwrap();
        assert_eq!(
            WorkspaceRoot(dir.0.clone()).read("bad.sysml"),
            Answer::not_implemented("reading a file that is not UTF-8")
        );
    }
}
