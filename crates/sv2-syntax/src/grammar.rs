// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Which of the two grammars a file is read against.
//!
//! `KerML` and `SysML` are two grammars over a shared vocabulary, not one grammar that
//! extends the other (ADR-0014). They disagree at the start symbol:
//!
//! ```text
//! KerML 8.2.3.4.1   RootNamespace = NamespaceBodyElement*
//! SysML 8.2.2.5.1   RootNamespace = PackageBodyElement*
//! ```
//!
//! Both are normative, each for its own language, so there is nothing to adjudicate and
//! no union to take: the grammar has to be chosen before the first token is read. A
//! grammar that accepted a `SysML` construct in a `KerML` file would pass a positive-only
//! corpus sweep, so that failure would be silent.
//!
//! The choice is made here and nowhere else. Per ADR-0015 everything downstream of it is
//! scope-free — a `PartUsage` is a `PartUsage` whichever file it came from — so no crate
//! above this one branches on the answer.

use std::path::Path;

/// The grammar a source file is read against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// `KerML`, the kernel modelling language. Files named `.kerml`.
    KerMl,
    /// `SysML` v2. A sibling of `KerML`, not an extension of it: at the grammar level
    /// `SysML.xtext` extends `KerMLExpressions`, not `KerML.xtext` (ADR-0010).
    SysMl,
}

impl Language {
    /// The grammar `path` is read against, or `None` when its name says neither.
    ///
    /// The single point at which the grammar is selected (ADR-0014, ADR-0015). Reading
    /// the extension is not I/O — nothing here opens the file, and `sv2-syntax` may not
    /// (STD-002-RS §2.5).
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "kerml" => Some(Self::KerMl),
            "sysml" => Some(Self::SysMl),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_suffix_chooses_the_grammar() {
        assert_eq!(
            Language::from_path(Path::new("a/b/Classes.kerml")),
            Some(Language::KerMl)
        );
        assert_eq!(
            Language::from_path(Path::new("a/b/SimpleVehicleModel.sysml")),
            Some(Language::SysMl)
        );
    }

    #[test]
    fn a_file_naming_neither_grammar_selects_neither() {
        // The negative case: `from_path` must not have a default, because guessing
        // would read a file against a grammar its author never wrote it in.
        assert_eq!(Language::from_path(Path::new("notes.md")), None);
        assert_eq!(Language::from_path(Path::new("Makefile")), None);
        assert_eq!(Language::from_path(Path::new(".kerml")), None);
    }
}
