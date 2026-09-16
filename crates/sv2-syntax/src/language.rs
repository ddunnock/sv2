// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The rowan `Language` binding: how [`SyntaxKind`] crosses into and out of a tree.

use crate::generated::kinds::{ALL, SyntaxKind};

/// The language tag rowan parameterises its tree types with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sv2Language {}

impl rowan::Language for Sv2Language {
    type Kind = SyntaxKind;

    /// Recover a kind from the `u16` rowan stored.
    ///
    /// By index into [`ALL`], whose order is the enum's declaration order. The
    /// workspace forbids `unsafe_code`, so a transmute is unavailable — and it would
    /// be wrong regardless: a `u16` that names no variant has to be representable,
    /// and `Tombstone` is that representation rather than a variant never constructed.
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        ALL.get(raw.0 as usize)
            .copied()
            .unwrap_or(SyntaxKind::Tombstone)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// A node in the lossless tree.
pub type SyntaxNode = rowan::SyntaxNode<Sv2Language>;
/// A token in the lossless tree.
pub type SyntaxToken = rowan::SyntaxToken<Sv2Language>;
/// Either of the two.
pub type SyntaxElement = rowan::SyntaxElement<Sv2Language>;

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::Language as _;

    #[test]
    fn every_kind_survives_the_round_trip_through_rowan() {
        // The property the tree depends on: what goes in as a kind comes back as the
        // same kind. A shifted ALL table would silently relabel every node.
        for kind in ALL {
            let raw = Sv2Language::kind_to_raw(*kind);
            assert_eq!(Sv2Language::kind_from_raw(raw), *kind);
        }
    }

    #[test]
    fn an_out_of_range_raw_kind_is_a_tombstone_not_a_panic() {
        let raw = rowan::SyntaxKind(u16::MAX);
        assert_eq!(Sv2Language::kind_from_raw(raw), SyntaxKind::Tombstone);
    }

    #[test]
    fn all_is_indexed_by_discriminant() {
        // ALL is generated in declaration order; if the generator ever emits it in a
        // different order this catches it at the first and last entries.
        assert_eq!(ALL.first().copied(), Some(SyntaxKind::Whitespace));
        assert_eq!(ALL.last().copied(), Some(SyntaxKind::Tombstone));
    }
}
