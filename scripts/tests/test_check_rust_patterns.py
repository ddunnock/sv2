# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from pathlib import Path

import pytest

from check_rust_patterns import file_findings

PATH = Path("crates/x/src/dispatch.rs")


def messages(source, path=PATH):
    return [f.message for f in file_findings(path, source)]


CLEAN = """
use tracing::info;

#[tracing::instrument(skip_all, fields(capability = %request.capability))]
fn dispatch(request: &Request) {
    info!(capability = %request.capability, elapsed_ms, "dispatched");
    tracing::warn!("literal braces {{like this}} are not interpolation");
    let label = format!("{name}");  // formatting outside an event is fine
    // info!("{name} in a comment is not code");
    let s = "a string that says #[ignore] and mod utils;";
}

#[test]
#[ignore = "needs the pinned corpus"]
fn slow_sweep() {}

struct Dispatcher;
mod dispatch_table;
"""


def test_conforming_source_has_no_findings():
    assert messages(CLEAN) == []


@pytest.mark.parametrize(
    ("source", "expected"),
    [
        ("#[instrument]\nfn f(secret: &str) {}", "#[instrument] without skip_all"),
        ("#[tracing::instrument(fields(run = %id))]\nfn f() {}", "#[instrument] without skip_all"),
        ('fn f() { info!("dispatched {cap}"); }', "info! message interpolates values"),
        (
            'fn f() { tracing::error!(code, "failed: {}", err); }',
            "error! message interpolates values",
        ),
        ('fn f() { debug!(format!("run {id}")); }', "debug! message built with format!"),
        ("#[test]\n#[ignore]\nfn t() {}", "bare #[ignore]"),
        ("mod utils;", "module `utils` has a banned name"),
        ("mod conn_manager { }", "module `conn_manager` has a banned name"),
        ("struct EventHandler;", "type `EventHandler` has a banned name"),
        ("pub enum Common { A }", "type `Common` has a banned name"),
    ],
)
def test_forbidden_pattern_is_reported(source, expected):
    found = messages(source)
    assert any(expected in m for m in found), found


def test_banned_module_file_name_is_reported():
    assert any(
        "module file `helpers.rs`" in m for m in messages("", Path("crates/x/src/helpers.rs"))
    )


def test_findings_report_the_line():
    (finding,) = file_findings(PATH, "\n\n#[ignore]\nfn t() {}")
    assert finding.line == 3
