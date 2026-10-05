// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Work counters for the performance probe, compiled in only with the `counters` feature.
//!
//! The counters are the deterministic half of ADR-0024's measurement, kept per thread. Two of them,
//! `peeked` and `consumed`, are what the ratchet reads: how many tokens lookahead
//! inspected and how many the parser consumed. Their ratio is what an accidental rescan
//! inflates — the lookahead that walked by index was quadratic, and a count of tokens
//! peeked per token consumed would have shown it on the first large file.
//!
//! The rest are **attribution detail** (`docs/perf/roadmap.md`, Phase 0): exact counts
//! at the helpers a sampled profile credits only approximately, because release
//! inlining merges callers into their callees. They are informational and never gate.
//!
//! | Counter | Counted at | Read by |
//! |---|---|---|
//! | `keyword_lookups` | `lookahead::keyword` | Phase 2 |
//! | `keyword_entries` | table entries `keyword`'s binary search probes: at most 9 of the 173 | Phase 2 |
//! | `is_name` | `Parser::is_name` | Phase 2 |
//! | `nth_is_keyword` | `Parser::nth_is_keyword` | Phase 2 |
//! | `qualified_names` | `Parser::qualified_name_length` | Phase 4 |
//! | `skip_occurrence_usage_prefix` | the method of that name | Phase 4 |
//! | `skip_basic_usage_prefix` | the method of that name | Phase 4 |
//! | `skip_prefix_metadata` | the method of that name | Phase 4 |
//! | `skip_prefix_metadata_computed` | the same, when its cache did not have the answer | Phase 4 |
//! | `member_decisions` | `Parser::body_element`, once per member read | Phase 5 |
//! | `member_dispatch` | `Parser::at_member_element` | Phase 5 |
//! | `keyword_member_dispatch` | `Parser::at_sysml_keyword_member` | Phase 5 |
//!
//! Without the feature every function here is an empty inline body, so the parser the
//! editor runs carries no counter and no branch for them.

#[cfg(feature = "counters")]
use std::cell::Cell;

/// One thing a counter counts. The order is [`NAMES`]'s, and the probe's output order.
#[derive(Clone, Copy)]
pub(crate) enum Counter {
    Peeked,
    Consumed,
    KeywordLookups,
    KeywordEntries,
    IsName,
    NthIsKeyword,
    QualifiedNames,
    SkipOccurrenceUsagePrefix,
    SkipBasicUsagePrefix,
    SkipPrefixMetadata,
    SkipPrefixMetadataComputed,
    MemberDecisions,
    MemberDispatch,
    KeywordMemberDispatch,
}

/// How many counters there are, the two ratcheted ones included.
const COUNT: usize = 14;

/// How many are attribution detail: all but `peeked` and `consumed`.
pub(crate) const DETAIL: usize = COUNT - 2;

/// Each counter's name, in [`Counter`]'s order; the probe prints these.
const NAMES: [&str; COUNT] = [
    "peeked",
    "consumed",
    "keyword_lookups",
    "keyword_entries",
    "is_name",
    "nth_is_keyword",
    "qualified_names",
    "skip_occurrence_usage_prefix",
    "skip_basic_usage_prefix",
    "skip_prefix_metadata",
    "skip_prefix_metadata_computed",
    "member_decisions",
    "member_dispatch",
    "keyword_member_dispatch",
];

// Per thread, so a parse counts only its own work: `cargo test` parses on many threads
// at once, and a process-wide count would mix theirs into every assertion.
#[cfg(feature = "counters")]
thread_local! {
    static COUNTS: [Cell<u64>; COUNT] = const { [const { Cell::new(0) }; COUNT] };
}

/// The work one or more parses did, as counted since the last [`take_counters`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    /// Tokens lookahead inspected without consuming them.
    pub peeked: u64,
    /// Tokens the parser consumed into the tree, trivia excluded.
    pub consumed: u64,
    /// The attribution detail, by name, in the order the module documentation lists.
    pub detail: [(&'static str, u64); DETAIL],
}

/// Add `n` to `counter`.
#[inline]
#[cfg_attr(
    not(feature = "counters"),
    expect(unused_variables, reason = "no-op without the feature")
)]
pub(crate) fn count_by(counter: Counter, n: u64) {
    #[cfg(feature = "counters")]
    COUNTS.with(|counts| {
        if let Some(slot) = counts.get(counter as usize) {
            slot.set(slot.get().wrapping_add(n));
        }
    });
}

/// Add one to `counter`.
#[inline]
pub(crate) fn count(counter: Counter) {
    count_by(counter, 1);
}

/// Record that lookahead inspected one token.
#[inline]
pub(crate) fn peeked() {
    count(Counter::Peeked);
}

/// Record that the parser consumed one token.
#[inline]
pub(crate) fn consumed() {
    count(Counter::Consumed);
}

/// Whether this build counts at all: `false` without the `counters` feature, when
/// [`take_counters`] always returns zeros.
#[doc(hidden)]
#[must_use]
pub const fn counters_enabled() -> bool {
    cfg!(feature = "counters")
}

/// Every counter's value, each reset to zero.
#[cfg(feature = "counters")]
fn drain() -> [u64; COUNT] {
    COUNTS.with(|counts| std::array::from_fn(|i| counts.get(i).map_or(0, |slot| slot.replace(0))))
}

/// Without the feature there is nothing to drain.
#[cfg(not(feature = "counters"))]
const fn drain() -> [u64; COUNT] {
    [0; COUNT]
}

/// The counts this thread made since its last call, which resets them.
///
/// The counters are per thread, so a caller sees exactly the parses it ran itself, on
/// the thread it asks from.
#[doc(hidden)]
#[must_use]
pub fn take_counters() -> Counters {
    let values = drain();
    let mut detail = [("", 0_u64); DETAIL];
    for ((entry, name), value) in detail
        .iter_mut()
        .zip(NAMES.iter().skip(2))
        .zip(values.iter().skip(2))
    {
        *entry = (name, *value);
    }
    Counters {
        peeked: values[0],
        consumed: values[1],
        detail,
    }
}
