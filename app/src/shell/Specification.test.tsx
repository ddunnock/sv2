// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen, within } from "@testing-library/react";

import { type ElementHandle, ElementHandleSchema } from "@/contract/element-id";
import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/fixture-thermal-control";
import { createModelQueries } from "@/ipc/model-queries";
import { Specification } from "./Specification";
import { ServicesProvider } from "./services";

afterEach(cleanup);

function petname(id: string): ElementHandle {
  const parsed = ElementHandleSchema.safeParse({ kind: "petname", id });
  if (!parsed.success) {
    throw new Error("fixture does not satisfy ElementHandleSchema");
  }
  return parsed.data;
}

/** The Specification on the fixture, through the real parse path (§11 rule 5). */
function specification(id: string): void {
  const queries = createModelQueries(fixtureTransport(THERMAL_CONTROL), "fixture");
  render(
    <ServicesProvider services={{ queries, report: () => undefined }}>
      <Specification handle={petname(id)} />
    </ServicesProvider>,
  );
}

const HEATER = "maple-sunrise-314";
const CONTROLLER = "river-lantern-117";

describe("Specification (IX-05)", () => {
  test("the header gives the keyword, the name and the qualified name", async () => {
    specification(HEATER);
    const article = await screen.findByRole("article", { name: "Heater" });
    expect(article.textContent).toContain("«part def»");
    expect(article.textContent).toContain("ThermalControl::Heater");
  });

  test("General shows the owner, the implicit specialization and the file", async () => {
    specification(HEATER);
    const general = await screen.findByRole("region", { name: "General" });
    expect(general.textContent).toContain("ThermalControl");
    expect(general.textContent).toContain("(implicit) Part");
    expect(general.textContent).toContain("model/ThermalControl.sysml");
  });

  describe("an unresolved type (ADR-0002)", () => {
    test("is still a row, marked unresolved rather than hidden", async () => {
      specification(HEATER);
      const table = within(await screen.findByRole("region", { name: "Owned features" }));
      const row = table.getByRole("row", { name: /state/ });
      expect(row.textContent).toContain("HeaterState");
      expect(within(row).getByText("(unresolved)")).toBeDefined();
    });

    test("puts the element's diagnostic in a banner at the top (IX-10)", async () => {
      specification(HEATER);
      const problems = await screen.findByRole("list", { name: "Problems" });
      expect(problems.textContent).toContain("cannot resolve type `HeaterState`");
    });
  });

  test("owned features read as the mockup's table: kind, name, type, multiplicity", async () => {
    specification(CONTROLLER);
    const table = within(await screen.findByRole("region", { name: "Owned features" }));
    const row = table.getByRole("row", { name: /sensorIn/ });
    expect(row.textContent).toBe("portsensorIn~TempPort1");
  });

  test("relationships read from this element's end", async () => {
    specification(CONTROLLER);
    const relationships = await screen.findByRole("region", { name: "Relationships" });
    expect(relationships.textContent).toContain("SatisfiesTemperatureStability");
    expect(relationships.textContent).toContain("Usagescontroller");
  });

  test("documentation is shown as text", async () => {
    specification(CONTROLLER);
    const documentation = await screen.findByRole("region", { name: "Documentation" });
    expect(documentation.textContent).toContain("Closed-loop regulator for panel temperature.");
  });

  test("a clean element has no problems banner", async () => {
    specification(CONTROLLER);
    await screen.findByRole("article", { name: "ThermalController" });
    expect(screen.queryByRole("list", { name: "Problems" })).toBeNull();
  });

  test("an element the model does not contain says so", async () => {
    specification("fern-quarry-221");
    expect(await screen.findByText(/no longer in the model/)).toBeDefined();
  });
});
