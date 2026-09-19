// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, beforeEach, describe, expect, mock, spyOn, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { IslandBoundary } from "./IslandBoundary";

afterEach(cleanup);

// React logs every caught render error itself. Silenced here so a passing test
// prints nothing; the preload restores it after each test.
beforeEach(() => {
  spyOn(console, "error").mockImplementation(() => undefined);
});

/** A component that fails to render while `broken.value` is true. */
const broken: { value: boolean } = { value: true };
function Fragile(): React.JSX.Element {
  if (broken.value) {
    throw new Error("render defect");
  }
  return <p>diagram content</p>;
}

describe("IslandBoundary", () => {
  test("a failing island shows an alert in its place and reports which island failed", () => {
    broken.value = true;
    const report = mock((_what: string, _detail: unknown) => undefined);
    render(
      <IslandBoundary island="diagram" report={report}>
        <Fragile />
      </IslandBoundary>,
    );
    expect(screen.getByRole("alert").textContent).toContain("The diagram stopped");
    expect(report).toHaveBeenCalledTimes(1);
    expect(report.mock.calls[0]?.[0]).toBe("render failure in the diagram island");
  });

  test("the other islands keep working (§7.4)", () => {
    broken.value = true;
    render(
      <>
        <IslandBoundary island="diagram" report={() => undefined}>
          <Fragile />
        </IslandBoundary>
        <IslandBoundary island="editor" report={() => undefined}>
          <p>editor content</p>
        </IslandBoundary>
      </>,
    );
    expect(screen.getByText("editor content")).toBeDefined();
  });

  test("Try again renders the island afresh once the defect is gone", async () => {
    broken.value = true;
    render(
      <IslandBoundary island="diagram" report={() => undefined}>
        <Fragile />
      </IslandBoundary>,
    );
    broken.value = false;
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(screen.getByText("diagram content")).toBeDefined();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  test("the panel shows the user no internals: not the error's message", () => {
    broken.value = true;
    render(
      <IslandBoundary island="sidebar" report={() => undefined}>
        <Fragile />
      </IslandBoundary>,
    );
    expect(screen.getByRole("alert").textContent).not.toContain("render defect");
  });
});
