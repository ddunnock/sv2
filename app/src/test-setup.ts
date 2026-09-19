// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The test preload, named by `bunfig.toml`'s `[test] preload` and run before
 * any test file (STD-004-TS §13.5).
 *
 * It is one of the two files allowed directly under `src/`; everything else
 * there belongs to a layer (§2, rule 1). It does §13.5's three things and
 * nothing else:
 *
 * 1. Registers happy-dom's global DOM, so component tests have a document
 *    (§11 rule 2). `model/`, `contract/` and `wasm/` tests must still not touch
 *    it — those layers have no DOM in production.
 * 2. Replaces `fetch` with one that throws. The target is air-gapped
 *    (invariant 6, §11 rule 8), so a test that reaches for the network fails
 *    loudly rather than slowly, or worse, passes on a machine that has one.
 * 3. After every test, restores mocks and the system clock, so neither leaks
 *    from one test into the next.
 */

import { afterEach, mock, setSystemTime } from "bun:test";
import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();

globalThis.fetch = Object.assign(
  (): Promise<Response> => {
    throw new Error("network access in a test: the target environment is air-gapped (§11 rule 8)");
  },
  { preconnect: (): void => undefined },
);

afterEach(() => {
  mock.restore();
  // Called with no argument, setSystemTime returns the clock to real time.
  setSystemTime();
});
