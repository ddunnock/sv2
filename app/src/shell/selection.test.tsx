// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { act, cleanup, render, renderHook } from "@testing-library/react";

import { type ElementHandle, ElementHandleSchema } from "@/contract/element-id";

import { SelectionProvider, useSelection } from "./selection";

afterEach(cleanup);

function handle(id: string): ElementHandle {
  const parsed = ElementHandleSchema.safeParse({ kind: "petname", id });
  if (!parsed.success) {
    throw new Error("fixture does not satisfy ElementHandleSchema");
  }
  return parsed.data;
}

describe("useSelection", () => {
  test("starts with nothing selected, and selecting sets it", () => {
    const { result } = renderHook(() => useSelection(), { wrapper: SelectionProvider });
    expect(result.current.selected).toBeNull();
    act(() => {
      result.current.select(handle("maple-sunrise-314"));
    });
    expect(result.current.selected).toEqual(handle("maple-sunrise-314"));
  });

  test("selecting an equal handle keeps the current value, so a re-fetch does not churn", () => {
    const { result } = renderHook(() => useSelection(), { wrapper: SelectionProvider });
    const first = handle("maple-sunrise-314");
    act(() => {
      result.current.select(first);
    });
    act(() => {
      result.current.select(handle("maple-sunrise-314"));
    });
    expect(result.current.selected).toBe(first);
  });

  test("selecting null clears it", () => {
    const { result } = renderHook(() => useSelection(), { wrapper: SelectionProvider });
    act(() => {
      result.current.select(handle("maple-sunrise-314"));
    });
    act(() => {
      result.current.select(null);
    });
    expect(result.current.selected).toBeNull();
  });

  describe("outside a provider", () => {
    beforeEach(() => {
      spyOn(console, "error").mockImplementation(() => undefined);
    });

    test("throws: a missing provider is a wiring defect, not a silent default", () => {
      function Orphan(): React.JSX.Element {
        useSelection();
        return <p>unreachable</p>;
      }
      expect(() => render(<Orphan />)).toThrow("outside a SelectionProvider");
    });
  });
});
