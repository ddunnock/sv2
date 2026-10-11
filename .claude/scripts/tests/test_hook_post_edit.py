# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""What the PostToolUse hook will hand to rustfmt, from a payload it does not control."""

import pytest

from hook_post_edit import edited_file


@pytest.fixture
def project(tmp_path, monkeypatch):
    """A project directory the hook has already changed into."""
    (tmp_path / "crates" / "sv2-syntax" / "src").mkdir(parents=True)
    (tmp_path / "crates" / "sv2-syntax" / "src" / "lib.rs").write_text("fn main() {}\n")
    monkeypatch.chdir(tmp_path)
    return tmp_path


def test_a_rust_file_in_the_project_is_accepted(project):
    assert edited_file("crates/sv2-syntax/src/lib.rs") == project / "crates/sv2-syntax/src/lib.rs"


def test_an_absolute_path_in_the_project_is_accepted(project):
    path = project / "crates/sv2-syntax/src/lib.rs"
    assert edited_file(str(path)) == path


@pytest.mark.parametrize(
    "path",
    [
        "--config-path=/tmp/evil.rs",
        "--emit=files.rs",
        "-Zunstable.rs",
    ],
)
@pytest.mark.usefixtures("project")
def test_an_argument_disguised_as_a_file_is_refused(path):
    assert edited_file(path) is None


@pytest.mark.usefixtures("project")
def test_a_file_outside_the_project_is_refused(tmp_path_factory):
    outside = tmp_path_factory.mktemp("elsewhere") / "other.rs"
    outside.write_text("fn main() {}\n")
    assert edited_file(str(outside)) is None


@pytest.mark.usefixtures("project")
def test_a_path_escaping_the_project_is_refused():
    assert edited_file("crates/../../outside.rs") is None


@pytest.mark.usefixtures("project")
def test_a_path_that_is_not_a_file_is_refused():
    assert edited_file("crates/sv2-syntax/src") is None
    assert edited_file("crates/sv2-syntax/src/absent.rs") is None
