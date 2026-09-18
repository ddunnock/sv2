# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_headers import MARKERS, WEB_EXCLUDED_PARTS, Settings, check, leading_comment, settings

SPDX = "SPDX-License-Identifier: MIT"
COPYRIGHT = "Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>"
REQUIRED = Settings(required=True, values=(SPDX, COPYRIGHT))
HEADER_PY = f'# {SPDX}\n# {COPYRIGHT}\n"""Doc."""\n'
HEADER_RS = f"// {SPDX}\n// {COPYRIGHT}\n//! Crate docs.\n"
HEADER_SH = f"#!/usr/bin/env bash\n# {SPDX}\n# {COPYRIGHT}\n# Purpose.\n"
# STD-004-TS §2.3: two `//` lines, then the TSDoc module comment. The block that
# follows is what makes the negative case below meaningful — the header must stop
# being collected at `/**`, or a licence inside a TSDoc block would pass.
HEADER_TS = f"// {SPDX}\n// {COPYRIGHT}\n/** What this module owns. */\nexport const a = 1;\n"
# The form §2.3 rule 1 forbids: TSDoc and editor hovers read `/** */`, so the licence
# would become documentation.
BLOCK_TS = f"/**\n * {SPDX}\n * {COPYRIGHT}\n */\nexport const a = 1;\n"


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
    ("name", "text"),
    [
        ("a.py", HEADER_PY),
        ("lib.rs", HEADER_RS),
        ("a.sh", HEADER_SH),
        ("a.ts", HEADER_TS),
        ("A.tsx", HEADER_TS),
        # An ambient declaration file is a source file and carries the header too.
        ("ambient.d.ts", HEADER_TS),
    ],
)
def test_correct_header_passes(tmp_path, name, text):
    assert check(write(tmp_path, name, text), REQUIRED) == []


@pytest.mark.parametrize("suffix", [".ts", ".tsx"])
def test_typescript_shares_the_rust_marker(suffix):
    """STD-004-TS §2.3 rule 1: two `//` lines, never a `/** */` block."""
    assert MARKERS[suffix] == "//"


def test_generated_is_the_only_excluded_web_directory():
    """§2 rule 4 — the generator owns a machine-written file, so it owns its header."""
    assert frozenset({"generated"}) == WEB_EXCLUDED_PARTS


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
        # STD-004-TS §2.3 rule 1. The licence is present and correctly spelled, and
        # the file still fails: a `/** */` opening is not a header, because TSDoc
        # would read it as documentation. Nothing starts the comment block, so this
        # is a missing header rather than a wrong one.
        ("Bad.tsx", BLOCK_TS, "no program header"),
        ("a.ts", HEADER_TS.replace("MIT", "GPL-3.0"), f"does not carry '{SPDX}'"),
        # Nothing may precede the header (§2.3), a triple-slash directive included.
        ("a.ts", '/// <reference types="bun" />\n' + HEADER_TS, "no program header"),
    ],
)
def test_header_violation_is_reported(tmp_path, name, text, expected):
    found = check(write(tmp_path, name, text), REQUIRED)
    assert any(expected in f for f in found), found


def test_shebang_is_rejected_even_when_headers_are_not_required(tmp_path):
    path = write(tmp_path, "a.py", "#!/usr/bin/env python3\n")
    assert check(path, Settings(required=False, values=())) != []
