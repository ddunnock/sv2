# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Writing a derived artifact: the one way the generators replace a file they own.

Every file under this module's care is produced from a pinned input and can be rebuilt by
rerunning its generator, so the risk is not losing the content but publishing a half
written version of it: a plain write truncates the file first, and a reader that arrives
in between — the next gate step, or the next session — sees an empty or partial artifact
with nothing to mark it as partial.

The same function exists as ``write_derived`` in ``.claude/scripts/_state.py``. The
duplication is deliberate: STD-001-PY §2 keeps the two script trees from sharing code,
and a three-line primitive is the right thing to copy across that line.
"""

from __future__ import annotations

from pathlib import Path


def write_derived(path: str | Path, text: str) -> None:
    """Replace ``path`` with ``text`` in one step, through a scratch file beside it.

    The scratch file is a sibling because ``Path.replace`` is only atomic within one
    filesystem. The encoding is named rather than left to the locale: a derived artifact
    is compared byte for byte by the gate, on two machines.
    """
    target = Path(path)
    scratch = target.with_name(f"{target.name}.new")
    scratch.write_text(text, encoding="utf-8")
    scratch.replace(target)
