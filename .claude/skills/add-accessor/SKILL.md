---
name: add-accessor
description: Implement one typed accessor in sv2-ast end to end — accessor, `// node:` marker, a positive case from the corpus, a named malformed case, the differential assertion, coverage and the gate. Use when adding an accessor to sv2-ast, when the user says "add an accessor" or names a node kind, or when working the ast-coverage worklist.
---

# Add an accessor

One accessor per invocation. If the request names several, do the first and say what
remains: batching is how half-built accessors get marked complete.

## 1. Take the next kind off the worklist

```bash
python3.12 -c "
import json
d = json.load(open('.claude/state/ast-coverage.json'))
print('\n'.join(d['unimplemented_kinds'][:10]))
"
```

The order is root downward and **advisory** (ADR-0025). Taking a kind out of order is
fine; say why.

## 2. Find a real example before writing code

The kind must occur in a real file. Expectations come from the specification or the
corpus, never from output (`.claude/rules/tests.md`).

```bash
grep -rln "<a keyword the kind starts with>" vendor/corpus | head -3
```

If no corpus file contains it, say so — that is a signal about whether the accessor is
worth writing yet, not a reason to invent an example.

## 3. Write the failing tests first

Both of them, before the accessor exists:

- the positive case, reading the real file found in step 2;
- the malformed case in `tests/malformed/<condition>.sysml`, named for the condition,
  with a header comment saying what it violates — and asserting all three parts: the
  parse yields a tree, the accessor reports absence rather than guessing, the neighbours
  still answer.

Run them. They must fail for the right reason.

## 4. Implement

The accessor wraps one `SyntaxNode`, carries `// node: Kind`, and returns nodes and
tokens — never a `String`. The differential assertion re-derives the same fact by walking
children directly.

## 5. Verify, in this order

```bash
python3.12 scripts/ast_coverage.py          # the kind moves to implemented
python3.12 scripts/check_ast_leaks.py       # owns no data, marker present
cargo test -p sv2-ast                       # positive, malformed, sweep, properties
./scripts/gate.sh                           # the definition of done
```

A sweep failure on a tree you did not write is the point of the sweep. Read the tree
before changing the accessor.

## 6. Commit

The accessor, both tests, the malformed data file and the regenerated report, in one
commit. An accessor without its malformed case is not finished.
