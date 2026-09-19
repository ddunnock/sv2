# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Assert that the webview's dependencies match the closed allowlist.

STD-004-TS §3.1 is the rule this enforces: *every dependency is a recorded
decision, and nothing arrives transitively as a direct import.* The allowlist in
``app/allowed-dependencies.toml`` is the machine-readable half of the table in
that section, and this is what compares it to ``app/package.json``.

    python3.12 scripts/check_web_dependencies.py

The two tables in the allowlist are deliberately different lists. ``[allowed]``
is what MAY be depended on — a decision. ``[present]`` is what currently IS — a
state. React and Tailwind are both decided; only some of what is decided has
been installed, because a package nothing imports is load cost and supply-chain
surface for nothing. So this checks three separate things: nothing is declared
that was never allowed, nothing is allowed under a spelling that permits drift,
and ``[present]`` still describes the manifest.

Every dependency field is treated alike. A range in ``peerDependencies`` is a
range (§3.1 rule 2), and the §13.1 manifest has no such field at all, so reading
only ``dependencies`` would leave the one place a range can hide unchecked.

This is the second row of the §13.7 enforcement-gap table.
"""

from __future__ import annotations

import argparse
import json
import os
import tomllib
from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_JSON = Path("app/package.json")
ALLOWLIST = Path("app/allowed-dependencies.toml")
# Every field npm and Bun read as a dependency declaration. `peerDependencies` is
# here because Bun resolves and installs one, so a range there is a real version
# the build takes, not documentation.
DEPENDENCY_FIELDS = (
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
)
# An exact version is three dot-separated numbers and nothing else. `file:` is the
# one other spelling the standard uses: §13.1 declares `sv2-wasm` as
# `file:../crates/sv2-wasm/pkg`, which names a path rather than a version and so
# cannot drift the way a range can.
FILE_SPEC = "file:"
# major.minor.patch, and nothing else. Anything shorter is a range with the tail
# left off: `19.3` accepts every 19.3.x, which is what rule 2 is about.
SEMVER_PARTS = 3


@dataclass(frozen=True, slots=True)
class Manifest:
    """The parts of package.json this checker reads."""

    declared: dict[str, str]
    trusted: tuple[str, ...]
    package_manager: str


def manifest(text: str) -> Manifest:
    """Read the dependency declarations, flattened across every field."""
    data = json.loads(text)
    declared: dict[str, str] = {}
    for field in DEPENDENCY_FIELDS:
        for name, spec in data.get(field, {}).items():
            declared[name] = str(spec)
    return Manifest(
        declared=declared,
        trusted=tuple(str(v) for v in data.get("trustedDependencies", [])),
        package_manager=str(data.get("packageManager", "")),
    )


def allowlist(text: str) -> tuple[frozenset[str], dict[str, str]]:
    """The ``[allowed]`` patterns and the ``[present]`` table."""
    data = tomllib.loads(text)
    allowed = frozenset(data.get("allowed", {}))
    present = {name: str(version) for name, version in data.get("present", {}).items()}
    return allowed, present


def is_allowed(name: str, patterns: frozenset[str]) -> bool:
    """Whether ``name`` is named by the allowlist, directly or by a scope glob."""
    return name in patterns or any("*" in p and fnmatch(name, p) for p in patterns)


def is_exact(spec: str) -> bool:
    """Whether ``spec`` pins one version (§3.1 rule 2) or names a path."""
    if spec.startswith(FILE_SPEC):
        return True
    parts = spec.split(".")
    return len(parts) == SEMVER_PARTS and all(part.isdigit() for part in parts)


def check(pkg: Manifest, allowed: frozenset[str], present: dict[str, str]) -> list[str]:
    """Every way the manifest and the allowlist disagree."""
    findings: list[str] = []

    for name, spec in sorted(pkg.declared.items()):
        if not is_allowed(name, allowed):
            findings.append(f"{name}: declared in package.json but not in [allowed]")
        if not is_exact(spec):
            findings.append(f"{name}: version {spec!r} is not exact; §3.1 rule 2 forbids ranges")

    findings.extend(
        f"{name}: in [present] but not in [allowed]"
        for name in sorted(present)
        if not is_allowed(name, allowed)
    )

    # [present] is a claim about the manifest, so it is wrong in both directions.
    for name, spec in sorted(pkg.declared.items()):
        if name not in present:
            findings.append(f"{name}: declared in package.json but missing from [present]")
        elif present[name] != spec:
            findings.append(f"{name}: [present] says {present[name]!r}, package.json says {spec!r}")
    findings.extend(
        f"{name}: in [present] but not declared in package.json"
        for name in sorted(present)
        if name not in pkg.declared
    )

    if pkg.trusted:
        findings.append(
            f"trustedDependencies is {list(pkg.trusted)}; §13.1 keeps it empty, "
            "because a trusted dependency runs install scripts"
        )
    if not pkg.package_manager.startswith("bun@") or not is_exact(
        pkg.package_manager.removeprefix("bun@")
    ):
        findings.append(
            f"packageManager is {pkg.package_manager!r}; §3.1 rule 5 pins Bun to an exact release"
        )
    return findings


def main(argv: list[str] | None = None) -> int:
    """Compare the manifest to the allowlist; 1 if they disagree."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    pkg = manifest(PACKAGE_JSON.read_text(encoding="utf-8"))
    allowed, present = allowlist(ALLOWLIST.read_text(encoding="utf-8"))
    findings = check(pkg, allowed, present)
    for finding in findings:
        print(f"  {finding}")
    if findings:
        print(f"web dependencies: {len(findings)} finding(s)")
        return 1
    print(
        f"web dependencies: {len(pkg.declared)} declared, all allowed, exact, "
        f"and matching [present]"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
