# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from check_standards_config import drift, python_config, rust_config

RUST_DOC = """## 13. Enforcement configuration

### 13.1 Workspace Cargo.toml

```toml
[workspace.package]
edition = "2024"
```

A member manifest:

```toml
[package]
name = "x"
```

### 13.4 rust-toolchain.toml

```toml
[toolchain]
channel = "1.98"
```
"""

PY_DOC = """```toml
[tool.ruff]
line-length = 100

[project.scripts]
harness = "harness_cli.cli:main"
```

```toml
this is = not [valid toml
```
"""


def test_blocks_bind_to_the_file_named_by_their_heading():
    assert rust_config(RUST_DOC) == {
        "Cargo.toml": {"workspace": {"package": {"edition": "2024"}}},
        "rust-toolchain.toml": {"toolchain": {"channel": "1.98"}},
    }


def test_python_blocks_keep_only_tool_tables_and_skip_fragments():
    assert python_config(PY_DOC) == {"tool": {"ruff": {"line-length": 100}}}


def test_changed_value_is_drift():
    assert drift({"a": {"b": 1}}, {"a": {"b": 2}}, "", "f.toml") == [
        ("a.b", "standard 1, repository 2")
    ]


def test_absent_key_is_drift():
    assert drift({"a": {"b": 1}}, {"a": {}}, "", "f.toml") == [("a.b", "absent from f.toml")]


def test_keys_the_repository_adds_are_not_drift():
    assert drift({"a": {"b": 1}}, {"a": {"b": 1, "c": 2}}, "", "f.toml") == []
