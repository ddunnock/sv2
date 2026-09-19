# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Check every workspace member against the manifest and layout rules of STD-002-RS.

    python3.12 scripts/check_rust_workspace.py

Checks (STD-002-RS §13.6 table):
  - every member inherits the workspace lint table (§2 rule 1)
  - every member takes edition, rust-version, license, and publish from the
    workspace (§2 rule 2), and does not rename its library (§2 rule 3)
  - no member declares its own version of a dependency (§12 rule 2)
  - only crates listed in scripts/rust_binaries.toml have a binary target (§2.2)
  - every lib.rs contains only docs, attributes, `mod`, and `pub use` (§2.1)

Reads `cargo metadata --no-deps --offline`, so it never resolves or fetches
dependencies. Inert when cargo is not installed or the workspace has no members.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST = Path("scripts/rust_binaries.toml")

# Crates nothing in the workspace may depend on (ADR-0018). An artifact, not a
# library: sv2-wasm is loaded BY the webview, built for wasm32-unknown-unknown, and
# linking it into the host binary would put a second parser in the same process as
# the first. cargo-deny cannot express this — an empty `wrappers` list bans the
# crate on its own existence — so deny.toml carries the direction it can check and
# this carries the one it cannot.
LEAF_CRATES = {"sv2-wasm": "an artifact the webview loads, not a library to link"}
INHERITED_KEYS = ("edition", "rust-version", "license", "publish")
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
# A dependency may add these to what it inherits; anything else declares its own source.
INHERITABLE_DEPENDENCY_KEYS = frozenset(("workspace", "features", "optional", "default-features"))

EXCERPT = 60

LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
ATTRIBUTE = re.compile(r"#!?\[[^\]]*\]")
LIB_STATEMENT = re.compile(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+|pub\s+use\s+[^;]+")

# Any: manifests and cargo metadata are deserialized documents (STD-001-PY §6).
Json = Any


@dataclass(frozen=True, slots=True)
class Finding:
    """One rule violation in one crate."""

    crate: str
    message: str
    section: str

    def render(self) -> str:
        """The finding as one `crate: message (section)` line."""
        return f"{self.crate}: {self.message} (STD-002-RS {self.section})"


def manifest_findings(crate: str, manifest: Json) -> list[Finding]:
    """Inheritance and dependency rules for one member manifest."""
    found = []
    if manifest.get("lints") != {"workspace": True}:
        found.append(Finding(crate, "[lints] must be exactly `workspace = true`", "§2 rule 1"))
    package = manifest.get("package", {})
    found.extend(
        Finding(crate, f"`{key}` must be `{key}.workspace = true`", "§2 rule 2")
        for key in INHERITED_KEYS
        if package.get(key) != {"workspace": True}
    )
    if "name" in manifest.get("lib", {}):
        found.append(Finding(crate, "[lib] must not override the crate name", "§2 rule 3"))
    for table, name, spec in _dependencies(manifest):
        if not isinstance(spec, dict) or spec.get("workspace") is not True:
            found.append(
                Finding(crate, f"[{table}] {name} must be `{{ workspace = true }}`", "§12 rule 2")
            )
        elif set(spec) - INHERITABLE_DEPENDENCY_KEYS:
            extra = ", ".join(sorted(set(spec) - INHERITABLE_DEPENDENCY_KEYS))
            found.append(Finding(crate, f"[{table}] {name} also declares {extra}", "§12 rule 2"))
    return found


def _dependencies(manifest: Json) -> list[tuple[str, str, Json]]:
    """(table, name, spec) for every dependency, including target-specific tables."""
    tables = [(t, manifest.get(t, {})) for t in DEPENDENCY_TABLES]
    for target, body in manifest.get("target", {}).items():
        tables.extend((f"target.{target}.{t}", body.get(t, {})) for t in DEPENDENCY_TABLES)
    return [(table, name, spec) for table, deps in tables for name, spec in deps.items()]


def leaf_findings(packages: list[Json]) -> list[Finding]:
    """A dependency on a crate that nothing may depend on (§2.5)."""
    return [
        Finding(
            package["name"],
            f"depends on `{dep['name']}`, which nothing may depend on: {LEAF_CRATES[dep['name']]}",
            "§2.5",
        )
        for package in packages
        for dep in package.get("dependencies", [])
        if dep["name"] in LEAF_CRATES
    ]


def binary_findings(packages: list[Json], allowed: dict[str, str]) -> list[Finding]:
    """A binary target in any crate the allowlist does not name."""
    return [
        Finding(package["name"], f"binary target `{target['name']}` is not allowed", "§2.2")
        for package in packages
        if package["name"] not in allowed
        for target in package["targets"]
        if "bin" in target["kind"]
    ]


def lib_findings(crate: str, lib_rs: str) -> list[Finding]:
    """Anything in a crate root other than docs, attributes, `mod`, and `pub use`."""
    text = ATTRIBUTE.sub("", BLOCK_COMMENT.sub("", LINE_COMMENT.sub("", lib_rs)))
    *statements, trailing = text.split(";")
    offending = [s.strip() for s in statements if not LIB_STATEMENT.fullmatch(s.strip())]
    if trailing.strip():
        offending.append(trailing.strip())
    return [
        Finding(crate, f"lib.rs contains `{_first_line(s)}`; only `mod` and `pub use`", "§2.1")
        for s in offending
    ]


def _first_line(statement: str) -> str:
    line = statement.splitlines()[0] if statement else ""
    return line if len(line) <= EXCERPT else line[: EXCERPT - 3] + "..."


def check_package(package: Json) -> list[Finding]:
    """Every per-crate rule for one workspace member."""
    manifest_path = Path(package["manifest_path"])
    found = manifest_findings(package["name"], tomllib.loads(manifest_path.read_text()))
    for target in package["targets"]:
        if "lib" in target["kind"] and Path(target["src_path"]).name == "lib.rs":
            found += lib_findings(package["name"], Path(target["src_path"]).read_text())
    return found


class MetadataError(Exception):
    """cargo could not describe the workspace, usually because a manifest is invalid."""


def workspace_members() -> list[Json] | None:
    """The workspace member packages from cargo metadata, or None if cargo is unavailable."""
    if shutil.which("cargo") is None:
        return None
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise MetadataError(result.stderr.strip())
    metadata = json.loads(result.stdout)
    members = set(metadata["workspace_members"])
    return [p for p in metadata["packages"] if p["id"] in members]


def main(argv: list[str] | None = None) -> int:
    """Check every workspace member; 1 if one breaks a manifest or layout rule."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    try:
        members = workspace_members()
    except MetadataError as exc:
        print(f"rust workspace: cargo metadata failed\n{exc}")
        return 1
    if members is None:
        print("rust workspace: cargo not on PATH — check inert")
        return 0
    if not members:
        print("rust workspace: no members — check inert")
        return 0

    allowed = tomllib.loads(ALLOWLIST.read_text()).get("crates", {})
    findings = binary_findings(members, allowed) + leaf_findings(members)
    for package in members:
        findings += check_package(package)

    for finding in findings:
        print(finding.render())
    if findings:
        print(f"rust workspace: {len(findings)} finding(s) in {len(members)} crate(s)")
        return 1
    print(f"rust workspace ok: {len(members)} crate(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
