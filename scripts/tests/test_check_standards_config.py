# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from check_standards_config import GOVERNED, declared, drift, normative_blocks

# One marked block per governed file, so these fixtures exercise a marker at a time
# without tripping the "every governed file must be declared" rule.
ALL_MARKED = "\n\n".join(f"```toml title={name}\nkey = 1\n```" for name in GOVERNED)

RUST_DOC = """## 13. Enforcement configuration

### 13.1 Workspace Cargo.toml

```toml title=Cargo.toml
[workspace.package]
edition = "2024"
```

A member manifest:

```toml
[package]
name = "x"
```

### 13.4 rust-toolchain.toml

```toml title=rust-toolchain.toml
[toolchain]
channel = "1.98"
```
"""


def _problems(text: str) -> list[str]:
    """The defects `declared` reports for one document, ignoring coverage of other files."""
    _, problems = declared({"doc.md": text})
    return [p for p in problems if "no normative block declares" not in p]


def test_a_marked_block_binds_to_the_file_its_fence_names():
    config, _ = declared({"doc.md": RUST_DOC})
    assert config == {
        "Cargo.toml": {"workspace": {"package": {"edition": "2024"}}},
        "rust-toolchain.toml": {"toolchain": {"channel": "1.98"}},
    }


def test_an_unmarked_block_is_an_illustration_and_is_not_read():
    # The member manifest's own [package] table, not the marked block's workspace.package.
    config, _ = declared({"doc.md": RUST_DOC})
    assert "package" not in config["Cargo.toml"]


def test_a_heading_naming_a_file_does_not_bind_an_unmarked_block():
    doc = "### 13.2 clippy.toml\n\n```toml\ncognitive-complexity-threshold = 40\n```\n"
    config, _ = declared({"doc.md": doc})
    assert config == {}


def test_blocks_for_one_file_merge():
    doc = "```toml title=clippy.toml\na = 1\n```\n\n```toml title=clippy.toml\nb = 2\n```\n"
    config, _ = declared({"doc.md": doc})
    assert config == {"clippy.toml": {"a": 1, "b": 2}}


def test_a_marked_block_that_is_not_toml_is_a_problem():
    doc = ALL_MARKED + "\n\n```toml title=clippy.toml\nthis is = not [valid toml\n```\n"
    assert [p for p in _problems(doc) if "not valid TOML" in p]


def test_an_unmarked_block_that_is_not_toml_is_not_a_problem():
    doc = ALL_MARKED + "\n\n```toml\nthis is = not [valid toml\n```\n"
    assert _problems(doc) == []


def test_a_marker_naming_an_ungoverned_file_is_a_problem():
    doc = "```toml title=Cargo.lock\na = 1\n```\n"
    assert [p for p in _problems(doc) if "not one of the governed files" in p]


def test_a_governed_file_no_block_declares_is_a_problem():
    _, problems = declared({"doc.md": "```toml title=clippy.toml\na = 1\n```\n"})
    missing = [p for p in problems if "no normative block declares" in p]
    assert len(missing) == len(GOVERNED) - 1


def test_every_governed_file_declared_leaves_no_coverage_problem():
    _, problems = declared({"doc.md": ALL_MARKED})
    assert problems == []


def test_a_problem_names_the_standard_and_the_line():
    doc = "intro\n\n" + "```toml title=Cargo.lock\na = 1\n```\n"
    assert [p for p in _problems(doc) if p.startswith("doc.md:3:")]


def test_a_block_records_the_line_its_fence_is_on():
    blocks = normative_blocks("one\ntwo\n```toml title=deny.toml\na = 1\n```\n", "doc.md")
    assert [(b.target, b.line) for b in blocks] == [("deny.toml", 3)]


def test_changed_value_is_drift():
    assert drift({"a": {"b": 1}}, {"a": {"b": 2}}, "", "f.toml") == [
        ("a.b", "standard 1, repository 2")
    ]


def test_absent_key_is_drift():
    assert drift({"a": {"b": 1}}, {"a": {}}, "", "f.toml") == [("a.b", "absent from f.toml")]


def test_keys_the_repository_adds_are_not_drift():
    assert drift({"a": {"b": 1}}, {"a": {"b": 1, "c": 2}}, "", "f.toml") == []
