# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_shell_standard import check_file

GOOD = '#!/usr/bin/env bash\n# Purpose.\nset -euo pipefail\n\nmain() {\n  echo ok\n}\n\nmain "$@"\n'


def write(tmp_path, name, text, *, executable=True):
    path = tmp_path / name
    path.write_text(text)
    path.chmod(0o755 if executable else 0o644)
    return path


def messages(path):
    return [f.message for f in check_file(path)]


def test_conforming_executable_has_no_findings(tmp_path):
    assert messages(write(tmp_path, "good-script.sh", GOOD)) == []


@pytest.mark.parametrize(
    ("name", "text", "executable", "expected"),
    [
        ("a.sh", GOOD.replace("#!/usr/bin/env bash", "#!/bin/sh"), True, "line 1 must be exactly"),
        ("a.sh", GOOD, False, "lacks the executable bit"),
        (
            "a.sh",
            GOOD.replace("set -euo pipefail", "set -uo pipefail"),
            True,
            "must precede the first command",
        ),
        (
            "a.sh",
            GOOD.replace("echo ok", "python3 - <<'PY'\nprint(1)\nPY"),
            True,
            "embedded interpreter",
        ),
        ("a.sh", GOOD.replace("echo ok", "python3.12 -c 'print(1)'"), True, "embedded interpreter"),
        ("a.sh", GOOD.replace("echo ok", 'eval "$cmd"'), True, "`eval` is prohibited"),
        ("Bad_Name.sh", GOOD, True, "kebab-case"),
        (
            "a.sh",
            GOOD.replace("# Purpose.", "# shellcheck disable=SC2034"),
            True,
            "file-wide shellcheck",
        ),
        ("a.sh", GOOD + "\n" * 150, True, "longer than 150 lines"),
    ],
)
def test_violation_is_reported(tmp_path, name, text, executable, expected):
    found = messages(write(tmp_path, name, text, executable=executable))
    assert any(expected in m for m in found), found


def test_running_a_python_file_is_not_an_embedded_program(tmp_path):
    text = GOOD.replace("echo ok", 'exec python3.12 "${here}/hook_x.py"')
    assert messages(write(tmp_path, "shim.sh", text)) == []


@pytest.mark.parametrize(
    ("text", "executable", "expected"),
    [
        ("#!/usr/bin/env bash\nhelper() { :; }\n", False, "must not have a shebang"),
        ("helper() { :; }\n", True, "must not be executable"),
        ("set -e\nhelper() { :; }\n", False, "must not change shell options"),
    ],
)
def test_library_rules(tmp_path, text, executable, expected):
    found = messages(write(tmp_path, "_lib.sh", text, executable=executable))
    assert any(expected in m for m in found), found
