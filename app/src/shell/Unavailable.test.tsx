// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";

import type { UnavailableReason } from "@/contract/availability";

import { Unavailable } from "./Unavailable";

afterEach(cleanup);

describe("Unavailable", () => {
  test.each<[string, UnavailableReason, string]>([
    [
      "not-implemented names what is missing",
      { kind: "not-implemented", capability: "diagram layout" },
      "sv2 does not yet support diagram layout",
    ],
    ["no-workspace", { kind: "no-workspace" }, "needs an open workspace"],
    ["opening", { kind: "opening" }, "finished opening"],
    ["read-only", { kind: "read-only" }, "read-only"],
    ["not-found", { kind: "not-found" }, "no longer in the model"],
  ])("%s", (_name, because, phrase) => {
    const { container } = render(<Unavailable what="General View" because={because} />);
    expect(container.textContent).toContain("General View");
    expect(container.textContent).toContain(phrase);
  });

  test("opening is the one live status, because it is the one that will change on its own", () => {
    render(<Unavailable what="General View" because={{ kind: "opening" }} />);
    expect(screen.getByRole("status")).toBeDefined();
  });

  test("every other reason is static text, not a status that claims progress", () => {
    render(
      <Unavailable
        what="General View"
        because={{ kind: "not-implemented", capability: "diagram layout" }}
      />,
    );
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
});
