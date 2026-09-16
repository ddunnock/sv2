# Vendored grammars

Grammar inputs are pinned by `vendor/sources.lock.toml` (sha256 per file) and fetched
into `vendor/pilot/`, not placed here by hand. To pin or move a pin:

```bash
python3.11 scripts/vendor_sync.py                # fetch missing files, verify pinned ones
python3.11 scripts/vendor_sync.py --accept-new   # a reviewed upgrade
python3.11 scripts/vendor_verify.py              # offline check; the gate runs this
```

Record the source and revision in `docs/conformance-target.toml` at the same time.

## Rules

- These files are read-only to agents. The PreToolUse hook blocks edits.
- Pin a tag or a full commit sha, never a branch. A branch is not a pin.
- Any local change requires an entry in `.claude/state/deviations.json`, a
  `// LOCAL PATCH:` comment at the site, and a re-pin. See `.claude/rules/grammar.md`.

## Choosing a source

Grammars for SysML v2 and KerML exist in several forms: the Xtext grammar in the OMG
pilot implementation, community ANTLR4 ports, and a tree-sitter grammar. They are not
equivalent — the ANTLR ports in circulation are known to carry defects that reject the
specification's own examples, which is why the deviation register
(`.claude/state/deviations.json`) is a first-class, schema-checked file rather than an
afterthought.

Whichever you pick, record why in `deviations.json` under `source_selection`, including what
you know about its defects going in.
