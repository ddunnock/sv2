# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_headers import Settings, check, leading_comment, settings

SPDX = "SPDX-License-Identifier: MIT"
COPYRIGHT = "Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>"
REQUIRED = Settings(required=True, values=(SPDX, COPYRIGHT))
HEADER_PY = f'# {SPDX}\n# {COPYRIGHT}\n"""Doc."""\n'
HEADER_RS = f"// {SPDX}\n// {COPYRIGHT}\n//! Crate docs.\n"
HEADER_SH = f"#!/usr/bin/env bash\n# {SPDX}\n# {COPYRIGHT}\n# Purpose.\n"


def write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(text)
    return path


def test_settings_default_to_not_required():
    assert settings("[tool.ruff]\nline-length = 100\n") == Settings(required=False, values=())


def test_settings_collect_every_required_value():
    config = settings('[tool.sv2.headers]\nrequired = true\nmust_contain = ["X", "", "Y"]\n')
    assert config == Settings(required=True, values=("X", "Y"))


def test_rust_doc_comments_are_not_a_header():
    assert leading_comment("//! Crate docs.\nfn main() {}\n", "//") == ""


@pytest.mark.parametrize(
    ("name", "text"), [("a.py", HEADER_PY), ("lib.rs", HEADER_RS), ("a.sh", HEADER_SH)]
)
def test_correct_header_passes(tmp_path, name, text):
    assert check(write(tmp_path, name, text), REQUIRED) == []


@pytest.mark.parametrize(
    ("name", "text", "expected"),
    [
        ("a.py", '"""Doc."""\n', "no program header"),
        ("a.py", HEADER_PY.replace("MIT", "GPL-3.0"), f"does not carry '{SPDX}'"),
        ("lib.rs", "//! Crate docs.\n", "no program header"),
        ("a.py", "#!/usr/bin/env python3\n" + HEADER_PY, "shebang present"),
        # The shebang and purpose lines are a comment block, so this is a header
        # that does not carry the required strings rather than a missing one.
        ("a.sh", "#!/usr/bin/env bash\n# Purpose.\n", f"does not carry '{SPDX}'"),
    ],
)
def test_header_violation_is_reported(tmp_path, name, text, expected):
    found = check(write(tmp_path, name, text), REQUIRED)
    assert any(expected in f for f in found), found


def test_shebang_is_rejected_even_when_headers_are_not_required(tmp_path):
    path = write(tmp_path, "a.py", "#!/usr/bin/env python3\n")
    assert check(path, Settings(required=False, values=())) != []
