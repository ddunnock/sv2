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

**If the kind is in every file** — `RootNamespace` and the other structural kinds — there
is nothing to grep for and no single file to name. Say that it is universal, and let the
sweep be the positive case: it already reads all 610 trees, which is a stronger statement
than any one file. Pick a short corpus file only if you need a concrete example in a
comment.

## 3. Write the failing tests first

Both of them, before the accessor exists:

- the positive case, reading the real file found in step 2 — or, for a universal kind,
  the sweep assertions of step 4;
- the malformed case at `tests/malformed/<kind-in-kebab>/<condition>.sysml` — the kind
  names the DIRECTORY, the condition names the file (`root-namespace/unclosed-body.sysml`),
  with a header comment saying what it violates — and asserting all three parts.
  `.claude/rules/ast.md` states what the three are and when part 3 is legitimately vacant.
  The layout is not a preference: `scripts/ast_coverage.py` reads it, and an implemented
  kind with no non-empty case there fails the gate, as does a case that names no kind.

Run them. They must fail for the right reason.

### When the malformed shape is a recovery fact, not a spec fact

"Write the test first" and "expectations never come from output" collide here, and the
resolution is not to skip either. What the parser does with an unclosed body is not in
the specification — it is this implementation's recovery policy — so you cannot derive
the expected tree from a clause, and guessing it wastes a round.

Explore first, in a throwaway test you delete: parse the malformed text, print the shape,
learn what recovery actually did. That is reading the *parser's* behaviour to find out
what to pin, which is legitimate. What is forbidden is reading your *new accessor's*
output and calling it the expectation — that proves only that the code does what it does.

Then write the case citing **the parser** for the recovery outcome — the function and the
file — and the clause only for what the clause actually says. A recovery outcome has no
clause and usually no ADR; ADR-0004 supports only the shape, that an error node is just
another node. Never cite a clause for a recovery choice, and never cite ADR-0022, which is
about deviation diagnostics and says nothing about recovery.

## 4. Implement, and give the sweep teeth

The accessor wraps one `SyntaxNode`, carries `// node: Kind`, and returns nodes and
tokens — never a `String`.

**Extend `tests/sweep.rs` with assertions for this kind.** This is not optional, and it
is the step most easily skipped: the sweep walks every tree in the repository, so an
accessor that is not called from it is not swept at all, and the suite stays green
because it has nothing to check. Add, in `assert_<kind>_accessor`:

- the cast contract — `cast` succeeds exactly when `kind()` matches, and `syntax()`
  hands back the node it was cast from;
- the differential assertion — re-derive the same fact by walking the tree directly, then
  assert **equality**, not containment. A subset check (`children.contains(member)`)
  passes an accessor that returns nothing, one that reverses the order, and one that
  silently filters a kind out. Those are the defects a later edit actually introduces.
- if the accessor filters anything, the assertion must filter the same way and a comment
  must say why — forcing the decision to be written down rather than made by accident.

Then prove the assertion can fail. Mutate the accessor (return an empty iterator; reverse
the order; filter a kind out), confirm the sweep goes red, and restore it. An assertion
you have not seen fail is a guess.

## 5. Verify, in this order

```bash
python3.12 scripts/ast_coverage.py          # the kind moves to implemented
python3.12 scripts/ast_coverage.py --check  # and is proved: no `unproved`, no `absent`
python3.12 scripts/check_ast_leaks.py       # owns no data, marker present
cargo test -p sv2-ast                       # positive, malformed, sweep, properties
./scripts/gate.sh                           # the definition of done
```

`unproved` means the malformed case is missing or misnamed. Write the case or fix the
name; never widen the check.

A sweep failure on a tree you did not write is the point of the sweep. Read the tree
before changing the accessor.

## 6. Commit

The accessor, both tests, the malformed data file and the regenerated report, in one
commit. An accessor without its malformed case is not finished.
