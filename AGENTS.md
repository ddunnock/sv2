# Codex Bridge

This repository's working guidance already lives in the existing project files. Treat
them as the source of truth; this file only routes Codex to them.

## Start here

- Workspace working agreement and invariants: `CLAUDE.md`
- Project overview, setup, and the anti-drift model: `README.md`
- Grammar derivation boundaries and source authority: `docs/DERIVATION.md`
- Architecture decisions: `docs/adr/README.md` and the ADRs it indexes
- Language and implementation standards: `docs/standards/`
- Work state and machine-owned JSON: `.claude/state/README.md` and `.claude/state/state.json`

## Path-scoped rules

- `crates/sv2-syntax/**`, `crates/sv2-ast/**`: `.claude/rules/syntax.md`
- `crates/sv2-hir/**`: `.claude/rules/hir.md`
- `crates/sv2-resolve/**`: `.claude/rules/resolve.md`
- `tests/**`, `**/*.snap`: `.claude/rules/tests.md`
- `vendor/**`, `grammars/**`, `docs/conformance-target.toml`, `.claude/state/grammar/**`:
  `.claude/rules/grammar.md`
- Any grammar derivation, grammar sourcing, or "can this be derived?" question:
  `docs/DERIVATION.md` first, then the matching `.claude/rules/*.md`
- `.claude/state/grammar/units/**`, `.claude/scripts/grammar_*.py`,
  `.claude/scripts/grammar-preflight.sh`: `.claude/rules/derivation.md`

## Skills and hook intent

- `.codex/skills/` contains symlinks to the existing `.claude/skills/` directories, so
  the `SKILL.md` files stay single-sourced.
- Targeted review and adjudication prompts live in:
  - `.claude/agents/spec-conformance-reviewer.md`
  - `.claude/agents/gate-triage.md`
  - `.claude/agents/grammar-adjudicator.md`
  - `.claude/agents/grammar-unit-validator.md`
- `.claude/settings.json` is still the authoritative description of the Claude hooks.
  Codex does not execute that file directly, so honor the same intent manually:
  - do not hand-edit script-owned derived artifacts; regenerate them with the documented
    scripts
  - when a rule or skill names a validator or generator, run it
  - before ending substantial work, run `./scripts/gate.sh`

## Notes

- `CLAUDE.md` is the high-level working agreement; the path-scoped `.claude/rules/*.md`
  files carry the narrower implementation rules when working in their matching areas.
