# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

import hook_protect_paths
from hook_protect_paths import decide, repo_relative

ROOT = "/repo"


@pytest.mark.parametrize(
    ("path", "expected"),
    [
        ("/repo/vendor/a.xmi", "vendor/a.xmi"),
        ("./vendor/a.xmi", "vendor/a.xmi"),
        ("vendor/a.xmi", "vendor/a.xmi"),
        ("/repository/vendor/a.xmi", "/repository/vendor/a.xmi"),
    ],
)
def test_repo_relative_normalizes_every_path_form(path, expected):
    assert repo_relative(path, ROOT) == expected


@pytest.mark.parametrize(
    ("path", "blocked_by"),
    [
        ("/repo/vendor/sources.lock.toml", "Vendor lockfile"),
        ("vendor/omg/SysML/SysML.xmi", "Vendored upstream artifact"),
        ("vendor/spec-bnf/SysML-textual-bnf.kebnf", "Vendored upstream artifact"),
        ("crates/sv2-syntax/src/generated/x.rs", "Generated code"),
        (".claude/state/grammar/reference.json", "Script-owned. reference/ledger"),
        (".claude/state/grammar/keywords.json", "Derived from the pinned grammars"),
        (".claude/state/grammar/bnf-productions.json", "Derived from the pinned grammars"),
        (".claude/state/decisions.json", "Script-owned. Regenerate"),
        (".claude/state/wiki-receipts.json", "Wiki receipts"),
        (".claude/state/schema/state.schema.json", "State schema"),
        ("docs/conformance-target.toml", "Conformance pin"),
    ],
)
def test_protected_paths_are_blocked(path, blocked_by):
    _, message = decide({"file_path": path}, ROOT)
    assert message is not None
    assert message.startswith(blocked_by)


@pytest.mark.parametrize(
    "path",
    [".claude/state/grammar/units/Name.json", "crates/sv2-syntax/src/lib.rs", "/etc/vendor/omg/x"],
)
def test_ordinary_paths_are_allowed(path):
    assert decide({"file_path": path}, ROOT) == (path, None)


def test_notebook_path_is_checked_when_file_path_is_absent():
    _, message = decide({"notebook_path": "vendor/pilot/K.xtext"}, ROOT)
    assert message is not None


@pytest.fixture
def repo(tmp_path):
    state = tmp_path / ".claude/state/state.json"
    state.parent.mkdir(parents=True)
    state.write_text(json.dumps({"generated": {"gates": {}}, "authored": {"objective": "a"}}))
    return tmp_path


@pytest.mark.parametrize(
    ("tool_input", "blocked"),
    [
        ({"content": json.dumps({"generated": {"gates": {"x": "pass"}}, "authored": {}})}, True),
        (
            {"content": json.dumps({"generated": {"gates": {}}, "authored": {"objective": "b"}})},
            False,
        ),
        ({"old_string": "a", "new_string": '"generated": 1'}, False),
    ],
)
def test_state_json_generated_block_is_script_owned(repo, tool_input, blocked):
    tool_input = {"file_path": str(repo / ".claude/state/state.json"), **tool_input}
    _, message = decide(tool_input, str(repo))
    assert (message is not None) == blocked


def test_unexpected_error_fails_closed(monkeypatch, capsys):
    def boom():
        msg = "broken"
        raise RuntimeError(msg)

    monkeypatch.setattr(hook_protect_paths, "main", boom)
    assert hook_protect_paths.run() == 2
    assert "BLOCKED" in capsys.readouterr().err
