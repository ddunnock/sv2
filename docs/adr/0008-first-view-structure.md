---
title: "First view is structure, interconnection second"
status: accepted
date: 2026-09-15
deciders: [David]
---

# First view is structure, interconnection second

## Context and problem statement

"BDD/IBD equivalent" was the initial target. SysML v2 has no BDD or IBD; the nearest
constructs are General View, Interconnection View, and the definition rendering. What is
colloquially meant spans two closures.

## Decision drivers

- Closure size drives build size more than any other variable
- Definition rendering is independently useful
- Connector ends pull in feature chaining, which is among the harder resolution rules

## Considered options

1. Structure first — definitions, usages, compartments, containment, specialization
2. Structure and interconnection together
3. Requirements table first, as the smallest closure

## Decision outcome

Option 1, with interconnection as the immediate follow-on.

Shipping both at once roughly doubles the metaclass set and pulls in feature chaining for
connector ends. Definition-only rendering is a coherent deliverable that exercises the
whole pipeline end to end, which is what a first view is actually for.

### Consequences

Good: the first vertical slice is genuinely thin, and every phase gets exercised.

Bad: the first release does not show connections, which is what most people picture when
they say IBD. Manage that expectation explicitly.

Option 3 was rejected because a table exercises almost no geometry and would leave the
layout and edit paths unproven.

## Pros and cons

**Option 1** — thin slice through every phase; incomplete-looking first output.

**Option 2** — recognizable output sooner; roughly double the closure before anything ships.

**Option 3** — smallest closure; proves the least.
