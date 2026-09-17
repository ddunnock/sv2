# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 6. Assembles the reference grammar, hashes it, and appends a ledger entry.

Refuses unless every unit is verified and the oracle is clean.

    python3.11 .claude/scripts/grammar_freeze.py           freeze
    python3.11 .claude/scripts/grammar_freeze.py --check   verify the frozen grammar still matches
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from typing import TYPE_CHECKING

from _grammar import GRAMMAR, load_units, render_ebnf, shadowed
from _state import REPO_ROOT, load_json, pin_value, utc_now

if TYPE_CHECKING:
    from _state import Json

REFERENCE = GRAMMAR / "reference.json"
LEDGER = GRAMMAR / "ledger.json"
EBNF = GRAMMAR / "reference.ebnf"
ORACLE_KEYS = ("accepted", "missed", "caught", "leaked")
REFERENCE_NOTE = (
    "The reference grammar: a machine-readable definition of the SysML v2 / KerML textual "
    "notation, derived clause by clause with traceable evidence. The Rust parser is checked "
    "against it; it is not generated from it."
)
LEDGER_NOTE = (
    "Append-only. One entry per freeze, so a future release can be compared against exactly "
    "what was frozen before it."
)


def grammar_body(units: dict[str, Json]) -> dict[str, Json]:
    """The frozen form of every unit that has a rule, in key order.

    Keyed by unit key, so a production the two languages state differently is frozen
    as `Name@kerml` and `Name@sysml` and a rebase compares each against its own past.
    """
    return {
        name: {
            "rule": unit["rule"],
            "decision": unit.get("decision"),
            "metaclass": unit["inputs"].get("metaclass", ""),
            "fingerprint": unit["fingerprint"]["combined"],
        }
        for name, unit in sorted(units.items())
        if unit.get("rule")
    }


def blockers(units: dict[str, Json], oracle: Json) -> list[str]:
    """Every reason the grammar cannot be frozen yet."""
    found: list[str] = []
    unverified = [n for n, u in units.items() if u["status"] != "verified"]
    if unverified:
        found.append(f"{len(unverified)} unit(s) not verified: {', '.join(sorted(unverified)[:8])}")
    if both := shadowed(units):
        found.append(f"{len(both)} production(s) have a shared unit and a variant: {both[:8]}")
    if not oracle:
        found.append("oracle has not been run — python3.11 .claude/scripts/grammar_validate.py")
    elif oracle.get("missed") or oracle.get("leaked"):
        found.append(
            f"oracle not clean: {oracle.get('missed')} missed, {oracle.get('leaked')} leaked"
        )
    elif oracle.get("skipped"):
        # A language whose files were never looked at has not passed; it was not tested.
        found.append(f"oracle skipped {oracle.get('skipped')} file(s) in an unchecked language")
    return found


def check(units: dict[str, Json], body: dict[str, Json], digest: str) -> int:
    """Whether the frozen reference still matches the units."""
    if not REFERENCE.exists():
        # Inert before derivation starts. Red only once rules exist that a frozen
        # grammar should have captured — otherwise a fresh clone fails its own gate.
        derived = [u for u in units.values() if u.get("rule")]
        if not derived:
            print("no grammar derived yet — freeze check inert")
            return 0
        # A derivation in progress is not a failure. The derive-grammar skill puts the
        # checkpoint on the unit files precisely so a session can derive a batch and
        # stop; a check that went red on the first rule would mean no session that
        # touched phase 3 could ever end green, and the only way to work would be to
        # switch the gate off. It goes red the moment the work is complete instead:
        # every unit verified with no frozen reference is a finished derivation nobody
        # froze, which is exactly the state this check exists to catch.
        unverified = [n for n, u in units.items() if u["status"] != "verified"]
        if unverified:
            done = len(units) - len(unverified)
            print(
                f"derivation in progress: {done}/{len(units)} verified, "
                f"{len(derived)} with rules — freeze check inert"
            )
            return 0
        print(f"all {len(units)} unit(s) are verified and the grammar has never been frozen.")
        print(
            "Run python3.11 .claude/scripts/grammar_validate.py"
            " then python3.11 .claude/scripts/grammar_freeze.py"
        )
        return 1
    current = json.loads(REFERENCE.read_text())
    if current.get("grammar_sha256") != digest:
        print("frozen reference grammar no longer matches the units.")
        print(f"  frozen  {current.get('grammar_sha256', '?')[:16]}")
        print(f"  units   {digest[:16]}")
        print("Re-run python3.11 .claude/scripts/grammar_freeze.py after the units settle.")
        return 1
    print(f"reference grammar current ({len(body)} productions, {digest[:16]})")
    return 0


def freeze(body: dict[str, Json], digest: str, oracle: Json) -> None:
    """Write the reference grammar, its rendered EBNF, and a ledger entry."""
    frozen_utc = utc_now()
    entry = {
        "frozen_utc": frozen_utc,
        "omg_namespace": pin_value("tier_a_omg.namespace"),
        "pilot_revision": pin_value("tier_b_pilot.revision"),
        "productions": len(body),
        "grammar_sha256": digest,
        "oracle": {k: oracle.get(k) for k in ORACLE_KEYS},
    }
    reference = {
        "_generated_by": ".claude/scripts/grammar_freeze.py",
        "_note": REFERENCE_NOTE,
        **entry,
        "grammar": body,
    }
    REFERENCE.write_text(json.dumps(reference, indent=2) + "\n")

    entries = json.loads(LEDGER.read_text())["entries"] if LEDGER.exists() else []
    entries.append(entry)
    LEDGER.write_text(json.dumps({"_note": LEDGER_NOTE, "entries": entries}, indent=2) + "\n")

    EBNF.write_text("\n".join(render_ebnf(n, b["rule"]) for n, b in sorted(body.items())) + "\n")
    print(f"frozen: {len(body)} productions, sha256 {digest[:16]}")
    print(
        "        reference.json, reference.ebnf (rendered, for review only),"
        f" ledger entry {len(entries)}"
    )


def main(argv: list[str] | None = None) -> int:
    """Freeze the reference grammar; 1 unless every unit is verified and the oracle is clean."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify instead of freezing")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    units = {n: u for n, u in load_units().items() if u["status"] != "retired"}
    body = grammar_body(units)
    digest = hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()
    if args.check:
        return check(units, body, digest)

    oracle = load_json(GRAMMAR / "validation.json")
    found = blockers(units, oracle)
    if found:
        print("cannot freeze:")
        for reason in found:
            print(f"  {reason}")
        return 1
    freeze(body, digest, oracle)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
