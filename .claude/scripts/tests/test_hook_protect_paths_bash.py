# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

import hook_protect_paths
from hook_protect_paths import bash_violation, protection

ROOT = "/repo"

DENY = [
    # redirections
    ("echo x > vendor/sources.lock.toml", ROOT),
    ("printf '%s' x >> docs/conformance-target.toml", ROOT),
    ("cat notes &> .claude/state/coverage.json", ROOT),
    ("date >| .claude/state/schema/state.schema.json", ROOT),
    # mutating commands
    ("rm -f vendor/omg/SysML.xmi", ROOT),
    ("rm -rf vendor", ROOT),
    ("rm -rf .", ROOT),
    ("rm -rf crates/sv2-syntax/src/generated", ROOT),
    ("touch vendor/sources.lock.toml", ROOT),
    ("mkdir -p vendor/omg/extra", ROOT),
    ("echo x > crates/sv2-syntax/src/generated/tokens.rs", ROOT),
    ("mv tmp.toml docs/conformance-target.toml", ROOT),
    ("cp /tmp/x.json .claude/state/decisions.json", ROOT),
    ("sed -i 's/a/b/' docs/conformance-target.toml", ROOT),
    ("sed --in-place=.bak 's/a/b/' .claude/state/schema/state.schema.json", ROOT),
    ("tee .claude/state/grammar/keywords.json < x", ROOT),
    ("truncate -s 0 vendor/sources.lock.toml", ROOT),
    ("chmod 600 docs/conformance-target.toml", ROOT),
    ("dd if=/dev/zero of=vendor/pilot/K.xtext", ROOT),
    ("git checkout -- docs/conformance-target.toml", ROOT),
    ("git restore .claude/state/schema", ROOT),
    ("find vendor -name '*.xmi' -delete", ROOT),
    # absolute, ./ and cwd-relative forms
    ("rm /repo/vendor/sources.lock.toml", ROOT),
    ("rm ./vendor/sources.lock.toml", ROOT),
    ("rm sources.lock.toml", "/repo/vendor"),
    ("cd vendor && rm sources.lock.toml", ROOT),
    ("cd /repo/docs; sed -i 's/x/y/' conformance-target.toml", "/elsewhere"),
    # wrappers and inline programs
    ("sudo rm vendor/sources.lock.toml", ROOT),
    ("FOO=1 env BAR=2 rm vendor/sources.lock.toml", ROOT),
    ('python3.11 -c \'open("docs/conformance-target.toml", "w").write("x")\'', ROOT),
    (
        (
            "python3.11 - <<'PY'\nfrom pathlib import Path\n"
            "Path('docs/conformance-target.toml').write_text('')\nPY"
        ),
        ROOT,
    ),
    ("perl -pi -e 's/a/b/' .claude/state/schema/deviations.schema.json", ROOT),
    ("bash -c 'rm vendor/sources.lock.toml'", ROOT),
    ("ls && echo hi > vendor/pilot/K.xtext", ROOT),
    # launchers do not hide a writing tool
    ("uvx ruff format .", ROOT),
    ("uvx ruff check --fix .", ROOT),
    ("uvx --from shfmt-py shfmt -w .", ROOT),
    # unparseable quoting that still names a protected path
    ("echo 'unterminated > docs/conformance-target.toml", ROOT),
]

ALLOW = [
    ("cat vendor/sources.lock.toml", ROOT),
    ("grep -rn namespace docs/conformance-target.toml", ROOT),
    ("sha256sum vendor/omg/SysML/SysML.xmi", ROOT),
    ("sed -n 1,20p docs/conformance-target.toml", ROOT),
    ("find vendor -name '*.xmi'", ROOT),
    ("git diff -- docs/conformance-target.toml", ROOT),
    ("git add docs/conformance-target.toml && git commit -m 'Pin vendor/sources.lock.toml'", ROOT),
    ("head -5 .claude/state/schema/state.schema.json > /tmp/head.txt", ROOT),
    ("python3.11 scripts/vendor_sync.py --accept-new", ROOT),
    ("python3.11 .claude/scripts/regen_state.py", ROOT),
    ("./scripts/gate.sh 2>&1 | tail -20", ROOT),
    ("echo x > .claude/state/grammar/units/Name.json", ROOT),
    ("rm -rf target/debug", ROOT),
    ("rm /etc/vendor/sources.lock.toml", ROOT),
    ("cp vendor/sources.lock.toml /tmp/lock.bak", ROOT),
    ('echo done > "$TMPDIR/out"', ROOT),
    ("python3.11 -c 'print(1)'", ROOT),
    ("ls -la # rm vendor/sources.lock.toml", ROOT),
    ("cat > crates/sv2-cli/src/lib.rs <<'EOF'\n//! Docs.\nEOF", ROOT),
    ("echo 'fn main() {}' > crates/sv2-cli/src/main.rs", ROOT),
    ("mkdir -p crates/sv2-cli/src && touch crates/sv2-cli/Cargo.toml", ROOT),
    ("mkdir -p vendor/scratch-notes", ROOT),
    ("cat >> notes.md <<'EOF'\nsee vendor/sources.lock.toml\nEOF\npython3.11 -m pytest -q", ROOT),
    ('python3.11 -c \'from pathlib import Path; print(Path("/repo") / "pyproject.toml")\'', ROOT),
    ("python3.11 scripts/check_shell_standard.py .", ROOT),
    ("uvx ruff format --check .", ROOT),
    ("uvx ruff check . && uvx mypy", ROOT),
    ("uvx --from shellcheck-py shellcheck scripts/*.sh", ROOT),
    ("uvx --from shfmt-py shfmt -d scripts .claude/scripts", ROOT),
    ("uv run --with pytest pytest -q .", ROOT),
]


@pytest.mark.parametrize(("command", "cwd"), DENY)
def test_writes_to_protected_paths_are_denied(command, cwd):
    assert bash_violation(command, ROOT, cwd) is not None


@pytest.mark.parametrize(("command", "cwd"), ALLOW)
def test_reads_and_owning_scripts_are_allowed(command, cwd):
    assert bash_violation(command, ROOT, cwd) is None


@pytest.mark.parametrize(
    ("rel", "protected"),
    [
        ("vendor", True),
        (".", True),
        (".claude/state/grammar", True),
        (".claude/state/grammar/units/Name.json", False),
        ("crates/sv2-syntax", True),
        ("2", False),
        ("print(1)", False),
        ("crates/sv2-syntax/src", False),
        ("scripts/tools/src", False),
        ("crates/sv2-syntax/src/generated", True),
        ("crates/sv2-cli/src/lib.rs", False),
        ("crates/sv2-cli/Cargo.toml", False),
        ("docs/adr", False),
    ],
)
def test_protection_covers_ancestor_directories(rel, protected):
    assert (protection(rel) is not None) == protected


def run_hook(monkeypatch, capsys, payload):
    monkeypatch.setenv("CLAUDE_PROJECT_DIR", ROOT)
    monkeypatch.setattr("sys.stdin", __import__("io").StringIO(json.dumps(payload)))
    code = hook_protect_paths.run()
    return code, capsys.readouterr()


def test_denial_is_json_on_stdout_with_exit_zero(monkeypatch, capsys):
    payload = {
        "tool_name": "Bash",
        "cwd": ROOT,
        "tool_input": {"command": "rm vendor/sources.lock.toml"},
    }
    code, out = run_hook(monkeypatch, capsys, payload)
    decision = json.loads(out.out)["hookSpecificOutput"]
    assert code == 0
    assert decision["permissionDecision"] == "deny"
    assert decision["permissionDecisionReason"].startswith("BLOCKED (Bash): ")
    assert out.err == ""


def test_file_tool_denial_uses_the_same_channel(monkeypatch, capsys):
    payload = {"tool_name": "Edit", "tool_input": {"file_path": f"{ROOT}/vendor/sources.lock.toml"}}
    code, out = run_hook(monkeypatch, capsys, payload)
    assert code == 0
    assert (
        "Vendor lockfile" in json.loads(out.out)["hookSpecificOutput"]["permissionDecisionReason"]
    )


def test_allowed_call_prints_nothing(monkeypatch, capsys):
    payload = {
        "tool_name": "Bash",
        "cwd": ROOT,
        "tool_input": {"command": "cat vendor/sources.lock.toml"},
    }
    code, out = run_hook(monkeypatch, capsys, payload)
    assert (code, out.out, out.err) == (0, "", "")


def test_src_directory_is_protected_only_when_it_holds_generated_code(tmp_path):
    (tmp_path / "crates/sv2-syntax/src/generated").mkdir(parents=True)
    (tmp_path / "crates/sv2-cli/src").mkdir(parents=True)
    root = str(tmp_path)
    assert bash_violation("rm -rf crates/sv2-syntax/src", root, root) is not None
    assert bash_violation("rm -rf crates/sv2-cli/src", root, root) is None
    assert bash_violation("python3.11 edit.py scripts/tools/src", root, root) is None
