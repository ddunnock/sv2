// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The measuring half of ADR-0024: parse named files and print one JSON document.
//!
//! ```text
//! perf_probe counters <file>...            deterministic counts; needs `--features counters`
//! perf_probe timing <runs> <file>...       median and p95 wall-clock; release builds only
//! ```
//!
//! The file list comes from the caller (`scripts/perf.py`), so the probe does no path
//! discovery and has no opinion on the workload. Besides each file, the counters mode
//! measures the scaling case: the first file repeated `SCALE` times in one input, whose
//! work per byte against one copy's is the quadratic detector.
//!
//! The JSON is written by hand because the workspace has no serializer as a dependency,
//! and the shape is flat: names, integers, and one ratio.

use std::alloc::System;
use std::fmt::Write as _;
use std::process::ExitCode;
use std::time::Instant;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use sv2_syntax::{Language, counters_enabled, parse, take_counters};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// How many copies of the first file the scaling case parses as one input.
const SCALE: usize = 8;

/// Untimed parses of each file before its timed runs.
const WARMUP: usize = 3;

/// What one parse cost, deterministically.
struct Work {
    bytes: u64,
    allocations: u64,
    allocated_bytes: u64,
    peeked: u64,
    consumed: u64,
}

fn work_of(text: &str, language: Language) -> Work {
    let _ = take_counters();
    let region = Region::new(GLOBAL);
    let parsed = parse(text, language);
    let stats = region.change();
    drop(parsed);
    let counted = take_counters();
    Work {
        bytes: text.len() as u64,
        allocations: (stats.allocations + stats.reallocations) as u64,
        allocated_bytes: stats.bytes_allocated as u64,
        peeked: counted.peeked,
        consumed: counted.consumed,
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Peeked tokens per byte, the work the scaling ratio compares.
fn per_byte(work: &Work) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a ratio of counts well under 2^52; the precision is not the point"
    )]
    let ratio = work.peeked as f64 / work.bytes.max(1) as f64;
    ratio
}

fn counters(files: &[(String, String, Language)]) -> Result<String, String> {
    if !counters_enabled() {
        return Err("built without the `counters` feature; the counts would all be zero".into());
    }
    let mut out = String::from("{\"mode\":\"counters\",\"files\":{");
    for (i, (name, text, language)) in files.iter().enumerate() {
        let w = work_of(text, *language);
        let _ = write!(
            out,
            "{}{}:{{\"bytes\":{},\"allocations\":{},\"allocated_bytes\":{},\"peeked\":{},\"consumed\":{}}}",
            if i == 0 { "" } else { "," },
            json_string(name),
            w.bytes,
            w.allocations,
            w.allocated_bytes,
            w.peeked,
            w.consumed
        );
    }
    out.push('}');
    if let Some((_, text, language)) = files.first() {
        let one = work_of(text, *language);
        let many = work_of(&text.repeat(SCALE), *language);
        let _ = write!(
            out,
            ",\"scaling\":{{\"copies\":{SCALE},\"ratio\":{:.4}}}",
            per_byte(&many) / per_byte(&one).max(f64::MIN_POSITIVE)
        );
    }
    out.push('}');
    Ok(out)
}

fn timing(runs: usize, files: &[(String, String, Language)]) -> Result<String, String> {
    if cfg!(debug_assertions) {
        return Err("timing needs a release build; a debug build's times mean nothing".into());
    }
    if counters_enabled() {
        return Err("timing needs a build without `counters`; the counting would be timed".into());
    }
    let mut out = String::from("{\"mode\":\"timing\",\"files\":{");
    for (i, (name, text, language)) in files.iter().enumerate() {
        // Warm the caches and the allocator first, so p95 measures parsing rather than
        // the first touch of the text.
        for _ in 0..WARMUP {
            std::hint::black_box(parse(std::hint::black_box(text), *language));
        }
        let mut samples: Vec<u128> = (0..runs.max(1))
            .map(|_| {
                let start = Instant::now();
                std::hint::black_box(parse(std::hint::black_box(text), *language));
                start.elapsed().as_nanos()
            })
            .collect();
        samples.sort_unstable();
        let at = |q: usize| {
            samples
                .get((samples.len() - 1) * q / 100)
                .copied()
                .unwrap_or(0)
        };
        let _ = write!(
            out,
            "{}{}:{{\"bytes\":{},\"median_ns\":{},\"p95_ns\":{}}}",
            if i == 0 { "" } else { "," },
            json_string(name),
            text.len(),
            at(50),
            at(95)
        );
    }
    out.push_str("}}");
    Ok(out)
}

fn read(paths: &[String]) -> Result<Vec<(String, String, Language)>, String> {
    paths
        .iter()
        .map(|p| {
            let language = Language::from_path(std::path::Path::new(p))
                .ok_or_else(|| format!("{p}: not a .sysml or .kerml file"))?;
            let text = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
            Ok((p.clone(), text, language))
        })
        .collect()
}

fn run(args: &[String]) -> Result<String, String> {
    match args {
        [mode, files @ ..] if mode == "counters" => counters(&read(files)?),
        [mode, runs, files @ ..] if mode == "timing" => {
            let runs = runs.parse().map_err(|e| format!("runs {runs:?}: {e}"))?;
            timing(runs, &read(files)?)
        }
        _ => {
            Err("usage: perf_probe counters <file>... | perf_probe timing <runs> <file>...".into())
        }
    }
}

#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measuring tool's output is its whole job"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("perf_probe: {message}");
            ExitCode::from(2)
        }
    }
}
