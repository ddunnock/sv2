// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The test preload, named by `bunfig.toml`'s `[test] preload` and run before
 * any test file (STD-004-TS §13.5).
 *
 * It is one of the two files allowed directly under `src/`; everything else
 * there belongs to a layer (§2, rule 1).
 *
 * A STUB, and the two things it will own are stated rather than implied:
 *
 * 1. DOM registration, via `@happy-dom/global-registrator`, so component tests
 *    have a document. Not installed — it is on the allowlist (§3.1) but not yet
 *    a dependency, and registering a DOM that no test needs would be load cost
 *    for nothing.
 * 2. A network stub that FAILS rather than passes. A test that reaches the
 *    network is a test that depends on something outside the repository, and
 *    this project has no network at run time at all (invariant 6). The stub
 *    turns that into a loud test failure instead of a slow one.
 *
 * Both arrive with the first component test. Until then this file exists so the
 * preload path in bunfig.toml resolves, and so that the place they go is
 * already decided.
 */

export {};
