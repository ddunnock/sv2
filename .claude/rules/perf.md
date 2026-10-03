---
paths:
  - "crates/sv2-syntax/**"
  - "scripts/perf*"
  - "tests/perf-baseline.json"
---

# Parser performance (ADR-0024)

## The ratchet is always on

`scripts/perf.sh check` runs in the gate on deterministic counters: allocations, bytes
allocated, and tokens peeked over the corpus, plus a scaling ratio that catches rescans.
It fails on a total grown past 10%, one file past 50%, or scaling past 1.25.

When grammar work legitimately costs more — a new production that builds more nodes —
re-baseline deliberately:

```bash
scripts/perf.sh record --reason "<the production, and why it costs what it does>"
```

Never re-baseline to absorb a scaling failure. A ratio above 1.25 means something
rescans the input, and that is a defect whatever the reason for the change was.

## Optimization is a series

Work whose purpose is speed goes through the `optimize-parser` skill and
`scripts/perf.sh series`. Each step improves its primary metric or is declared enabling,
nothing worsens past noise, and the parse output — every tree, error and deviation note
over the corpus — stays identical.

## Fix the rule, never the check

The tolerances are named constants in `scripts/perf.py` with ADR-0024's reasons. Loosening
one to clear a gate is the same move as widening any other checker's allowed set here.
