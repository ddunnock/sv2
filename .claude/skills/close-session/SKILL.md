---
name: close-session
description: End a working session cleanly — run the gates, regenerate the derived JSON, write the authored block of state.json, and append the log entry. Use when the user says they are done, wrapping up, stopping for the day, or asks to close out or hand off.
---

# Close a session

The next session starts with a fresh context and reads `.claude/state/state.json`. It must
be able to resume from that file alone, without the transcript.

## 1. Gates first

```bash
./scripts/gate.sh
```

Red? Do not close out with a green-sounding summary. Either fix it, or record it in
`authored.pending_decisions` with what is broken and what you tried. A handoff that hides a
red gate costs the next session more than it saves this one.

## 2. Conformance review, if the session touched semantics

If the diff includes grammar handling, desugaring, implied specializations, or constraints,
run the `spec-conformance-reviewer` subagent. Unresolved blocking findings go into
`pending_decisions`.

## 3. Regenerate everything derived

```bash
python3.12 scripts/extract_productions.py
python3.12 scripts/bnf_coverage.py
python3.12 scripts/grammar_diff.py
python3.12 .claude/scripts/index_decisions.py
python3.12 .claude/scripts/regen_state.py
```

Never hand-write these. The PreToolUse hook blocks them, and the reason is that a
hand-maintained status file is a second source of truth that goes stale silently.

## 4. Write the authored object

Only you can write this — it is intent, and intent is not derivable. Edit
`.claude/state/state.json`, `authored` only:

- **`objective`** — still accurate, or did this session change it?
- **`next_step`** — exactly one, concrete enough to start cold. The schema enforces a
  minimum length precisely to stop `"TBD"` and `"continue the lexer"` from being accepted.
  Name the function and the input file.
- **`pending_decisions`** — add what surfaced, remove what got settled. Each needs `id`,
  `question`, and `blocks`.
- **`log`** — append one entry, newest last:

```json
{
  "date": "2026-09-15",
  "gate": "green",
  "summary": "implemented partDefinition and portDefinition; two negative cases for duplicate names; ADR-0009 still open"
}
```

Then validate, under the project `.venv`, which is where jsonschema is:

```bash
uv run python .claude/scripts/validate_state.py
```

It must print `state schemas valid (N units)`. Under the system `python3.12` it takes a
structural fallback that does not check types — a `blocks` written as a string rather
than an array passed it and failed the gate.

## 5. Deviations, if any were decided

New grammar differences resolved this session go into `.claude/state/deviations.json`, each
with at least one piece of evidence — a retrieved spec clause, a corpus file, or an OMG
issue. The schema enforces the evidence array; `grammar_diff.py` reads this file to tell
reviewed differences from unreviewed ones.

## 6. Report

Gate status, what changed, what the next step is. Three sentences. Do not re-summarize the
session — `python3.12 .claude/scripts/state_report.py` is the summary now.
