// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Work counters for the performance probe, compiled in only with the `counters` feature.
//!
//! The counters are the deterministic half of ADR-0024's measurement: how many tokens
//! lookahead inspected and how many the parser consumed. Their ratio is what an
//! accidental rescan inflates — the lookahead that walked by index was quadratic, and a
//! count of tokens peeked per token consumed would have shown it on the first large file.
//!
//! Without the feature every function here is an empty inline body, so the parser the
//! editor runs carries no atomic and no branch for them.

#[cfg(feature = "counters")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "counters")]
static PEEKED: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "counters")]
static CONSUMED: AtomicU64 = AtomicU64::new(0);

/// The work one or more parses did, as counted since the last [`take_counters`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    /// Tokens lookahead inspected without consuming them.
    pub peeked: u64,
    /// Tokens the parser consumed into the tree, trivia excluded.
    pub consumed: u64,
}

/// Record that lookahead inspected one token.
#[inline]
pub(crate) fn peeked() {
    #[cfg(feature = "counters")]
    PEEKED.fetch_add(1, Ordering::Relaxed);
}

/// Record that the parser consumed one token.
#[inline]
pub(crate) fn consumed() {
    #[cfg(feature = "counters")]
    CONSUMED.fetch_add(1, Ordering::Relaxed);
}

/// Whether this build counts at all: `false` without the `counters` feature, when
/// [`take_counters`] always returns zeros.
#[doc(hidden)]
#[must_use]
pub const fn counters_enabled() -> bool {
    cfg!(feature = "counters")
}

/// The counts since the last call, which resets them.
///
/// The counters are process-wide, so a caller that wants one parse's work parses on one
/// thread with nothing else parsing. That is the probe's situation, and the only one
/// this is for.
#[doc(hidden)]
#[must_use]
pub fn take_counters() -> Counters {
    #[cfg(feature = "counters")]
    {
        Counters {
            peeked: PEEKED.swap(0, Ordering::Relaxed),
            consumed: CONSUMED.swap(0, Ordering::Relaxed),
        }
    }
    #[cfg(not(feature = "counters"))]
    {
        Counters::default()
    }
}
