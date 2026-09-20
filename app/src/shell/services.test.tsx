// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { cleanup, render, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

import type { Answer } from "@/contract/availability";
import { NO_EDITOR_WINDOW } from "@/ipc/editor-window";
import type { ModelQueries, Reply } from "@/ipc/model-queries";
import { ok, type Result } from "@/model/result";

import { ServicesProvider, useAnswer, useServices } from "./services";

afterEach(cleanup);

/** A reply the test settles by hand, so the order of replies is under its control. */
function deferred<T>(): { promise: Reply<T>; settle: (answer: Answer<T>) => void } {
  let settle: (answer: Answer<T>) => void = () => undefined;
  const promise = new Promise<Result<Answer<T>, never>>((resolve) => {
    settle = (answer) => {
      resolve(ok(answer));
    };
  });
  return { promise, settle };
}

// The hook reaches the queries only through `ask`, so the services' queries are
// never called here; an empty object stands in for them honestly.
const SERVICES = {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the hook under test never touches queries; every question goes through the ask function the test supplies.
  queries: {} as ModelQueries,
  report: () => undefined,
  editorWindow: NO_EDITOR_WINDOW,
} as const;

const wrapper = ({ children }: Readonly<{ children: ReactNode }>): React.JSX.Element => (
  <ServicesProvider services={SERVICES}>{children}</ServicesProvider>
);

describe("useAnswer", () => {
  test("starts loading, then holds the answer", async () => {
    const reply = deferred<number>();
    const { result } = renderHook(() => useAnswer(() => reply.promise, "q"), { wrapper });
    expect(result.current).toEqual({ status: "loading", previous: null });
    reply.settle({ kind: "ready", data: 1 });
    await waitFor(() => {
      expect(result.current).toEqual({ status: "answered", answer: { kind: "ready", data: 1 } });
    });
  });

  test("a new key keeps the old answer on screen while it asks (ADR-0002)", async () => {
    const first = deferred<number>();
    const second = deferred<number>();
    const { result, rerender } = renderHook(
      ({ key }: { key: string }) =>
        useAnswer(() => (key === "a" ? first.promise : second.promise), key),
      { wrapper, initialProps: { key: "a" } },
    );
    first.settle({ kind: "ready", data: 1 });
    await waitFor(() => {
      expect(result.current.status).toBe("answered");
    });
    rerender({ key: "b" });
    expect(result.current).toEqual({ status: "loading", previous: { kind: "ready", data: 1 } });
  });

  test("a reply to a superseded question is ignored", async () => {
    const stale = deferred<number>();
    const fresh = deferred<number>();
    const { result, rerender } = renderHook(
      ({ key }: { key: string }) =>
        useAnswer(() => (key === "old" ? stale.promise : fresh.promise), key),
      { wrapper, initialProps: { key: "old" } },
    );
    rerender({ key: "new" });
    fresh.settle({ kind: "ready", data: 2 });
    await waitFor(() => {
      expect(result.current.status).toBe("answered");
    });
    // The superseded reply lands last. The hook must drop it, not overwrite.
    stale.settle({ kind: "ready", data: 1 });
    await stale.promise;
    // Give React a macrotask to render anything the stale reply scheduled;
    // without this the assertion runs before an overwrite could show.
    await new Promise((resolve) => {
      setTimeout(resolve, 20);
    });
    expect(result.current).toEqual({ status: "answered", answer: { kind: "ready", data: 2 } });
  });
});

describe("useServices", () => {
  beforeEach(() => {
    spyOn(console, "error").mockImplementation(() => undefined);
  });

  test("throws outside a provider: a missing service is a wiring defect", () => {
    function Orphan(): React.JSX.Element {
      useServices();
      return <p>unreachable</p>;
    }
    expect(() => render(<Orphan />)).toThrow("outside a ServicesProvider");
  });
});
