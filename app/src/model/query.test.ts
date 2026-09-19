// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import type { Answer } from "@/contract/availability";
import { answerToShow, firstLoad, type Query, reload } from "./query";

type Q = Query<number, string>;

const READY: Answer<number> = { kind: "ready", data: 42 };
const UNAVAILABLE: Answer<number> = {
  kind: "unavailable",
  reason: { kind: "not-implemented", capability: "name resolution" },
};

describe("Query", () => {
  test("a first load has nothing to show", () => {
    expect(answerToShow(firstLoad<number, string>())).toBeNull();
  });

  test("an answered query shows its answer, ready or unavailable", () => {
    expect(answerToShow<number, string>({ status: "answered", answer: READY })).toEqual(READY);
    expect(answerToShow<number, string>({ status: "answered", answer: UNAVAILABLE })).toEqual(
      UNAVAILABLE,
    );
  });

  test("reloading keeps showing the last answer, so nothing flickers (ADR-0002)", () => {
    const reloading = reload<number, string>({ status: "answered", answer: READY });
    expect(reloading.status).toBe("loading");
    expect(answerToShow(reloading)).toEqual(READY);
  });

  test("an unavailable answer is held through a reload too", () => {
    // not-implemented today is still the thing to show while asking again.
    expect(
      answerToShow(reload<number, string>({ status: "answered", answer: UNAVAILABLE })),
    ).toEqual(UNAVAILABLE);
  });

  test("reloading twice holds the same answer, not a nested one", () => {
    const once = reload<number, string>({ status: "answered", answer: READY });
    expect(reload(once)).toEqual(once);
  });

  test("reloading after a failure starts from nothing, because a failed read produced no answer", () => {
    const failed: Q = { status: "failed", error: "ipc:workspace" };
    expect(reload(failed)).toEqual(firstLoad());
  });

  test("a failed query shows no answer; the failure is reported, not drawn as data", () => {
    expect(answerToShow<number, string>({ status: "failed", error: "ipc:workspace" })).toBeNull();
  });
});
