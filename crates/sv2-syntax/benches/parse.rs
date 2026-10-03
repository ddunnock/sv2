// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Parse-time benchmarks, for people: exploring, comparing branches, and as the
//! flamegraph target (ADR-0024).
//!
//! ```bash
//! cargo bench -p sv2-syntax --bench parse
//! cargo flamegraph -p sv2-syntax --bench parse -- --bench simple_vehicle_model
//! ```
//!
//! Verdicts are not read from here. divan's table is for reading; the deterministic
//! counters and the timings `scripts/perf.sh` judges come from `examples/perf_probe.rs`.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use divan::Bencher;
use divan::counter::BytesCount;
use sv2_syntax::{Language, parse};

fn main() {
    divan::main();
}

/// The largest corpus file, which stands in for ADR-0013 FIT-4's "largest standard
/// library file" while the standard library is not vendored.
const LARGEST: &str = "vendor/corpus/omg/SimpleVehicleModel.sysml";

/// How deep the nesting case goes: deep, but under the parser's `MAX_DEPTH` of 400, so
/// it measures descent rather than the depth limit's report.
const NESTING: usize = 300;

/// How many terms the expression case chains.
const TERMS: usize = 2000;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every model file under `root`, sorted, with its text and language. Unreadable files are
/// skipped: a benchmark measures what is there.
fn model_files(root: &Path) -> Vec<(String, Language)> {
    let mut stack = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if Language::from_path(&path).is_some() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| Some((std::fs::read_to_string(&p).ok()?, Language::from_path(&p)?)))
        .collect()
}

static CORPUS: LazyLock<Vec<(String, Language)>> =
    LazyLock::new(|| model_files(&workspace().join("vendor/corpus")));

static LARGEST_TEXT: LazyLock<String> =
    LazyLock::new(|| std::fs::read_to_string(workspace().join(LARGEST)).unwrap_or_default());

/// The largest corpus file, parsed whole.
#[divan::bench]
fn simple_vehicle_model(bencher: Bencher) {
    let text = LARGEST_TEXT.as_str();
    bencher
        .counter(BytesCount::of_str(text))
        .bench(|| parse(divan::black_box(text), Language::SysMl));
}

/// Every corpus file, one after another: the mix of constructs the corpus has.
#[divan::bench(sample_count = 20)]
fn whole_corpus(bencher: Bencher) {
    let bytes: usize = CORPUS.iter().map(|(text, _)| text.len()).sum();
    bencher.counter(BytesCount::new(bytes)).bench(|| {
        for (text, language) in CORPUS.iter() {
            divan::black_box(parse(divan::black_box(text), *language));
        }
    });
}

/// The largest file repeated in one file. Time per byte should not grow with the copies;
/// if it does, something rescans.
#[divan::bench(args = [1, 4, 16])]
fn repeated(bencher: Bencher, copies: usize) {
    let text = LARGEST_TEXT.repeat(copies);
    bencher
        .counter(BytesCount::of_str(&text))
        .bench(|| parse(divan::black_box(&text), Language::SysMl));
}

/// Parts nested `NESTING` deep: recursive descent at depth.
#[divan::bench]
fn nested_parts(bencher: Bencher) {
    let text = format!(
        "package P {{ {}{} }}",
        "part a { ".repeat(NESTING),
        "}".repeat(NESTING)
    );
    bencher
        .counter(BytesCount::of_str(&text))
        .bench(|| parse(divan::black_box(&text), Language::SysMl));
}

/// One attribute whose value chains `TERMS` additions: the precedence climb at length.
#[divan::bench]
fn long_expression(bencher: Bencher) {
    let text = format!("package P {{ attribute x = 1{}; }}", " + 1".repeat(TERMS));
    bencher
        .counter(BytesCount::of_str(&text))
        .bench(|| parse(divan::black_box(&text), Language::SysMl));
}
