// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The mockup's sample model, `ThermalControl`, as raw replies.
 *
 * SAMPLE DATA, AND SAID SO. Everything here is the mockup's invented model
 * (docs/design/sysml-workbench-mockup, "Sample model"), and every petname,
 * offset and library UUID is invented too. It reaches the screen only behind
 * the status bar's "fixture data" indicator, which `provenance` drives.
 *
 * RAW, SO IT IS CHECKED. These are JSON-shaped values typed `unknown`, not
 * contract types: they go through the same schema a backend reply does, and a
 * fixture that drifts from the contract fails `fixture-client.test.ts`.
 *
 * `Heater` refers to `HeaterState`, which does not exist — the mockup's own
 * deliberate diagnostic. The code it carries, `RES-UNRESOLVED-NAME`, is not
 * one `sv2-syntax` raises (no `RES-*` code exists yet); it has the shape
 * `contract/diagnostic.ts` admits, which is all a fixture can honestly claim.
 */

import type { Fixtures } from "./fixture-client";

const ONE = { lower: null, upper: { kind: "literal", value: 1 } };
const NONE = { error: 0, warning: 0, info: 0 };

/** A resolved reference to a user element. */
function to(id: string, name: string): unknown {
  return { kind: "resolved", target: { kind: "petname", id }, name };
}

/** A resolved reference to a standard library element. */
function lib(uuid: string, name: string): unknown {
  return { kind: "resolved", target: { kind: "library", uuid }, name };
}

/** A feature row's parts: its identity, metaclass, name and one type. */
type FeatureSpec = Readonly<{ id: string; metaclass: string; name: string; type: unknown }>;

/** A feature row, owned, with multiplicity 1 as every row in the mockup has. */
function feature({ id, metaclass, name, type }: FeatureSpec): unknown {
  return {
    handle: { kind: "petname", id },
    metaclass,
    name,
    types: [type],
    multiplicity: ONE,
    origin: { kind: "owned" },
  };
}

const PACKAGE = to("cedar-harbor-208", "ThermalControl");
const TEMPERATURE_VALUE = lib("2d7e1c40-5a3b-5f6e-9a1d-4c8b7e6f5a21", "TemperatureValue");
const REAL = lib("7b3f9e12-0c4d-5a8b-8e2f-1a6d3c9b4e07", "Real");
const POWER_VALUE = lib("c1a8e5d3-9f2b-5c4e-b7a6-3e0d1f8c2b94", "PowerValue");
const PART = lib("4f6a2b8c-1d3e-5f7a-9b0c-2d4e6f8a0b1c", "Part");

/** `Parts::Part`, which every part definition specializes implicitly. */
const IMPLICIT_PART = {
  handle: { kind: "derived", hash: "a41f0c9e" },
  metaclass: "Subclassification",
  direction: "outgoing",
  other: PART,
  isImplied: true,
};

const THERMAL_CONTROLLER = {
  kind: "ready",
  data: {
    summary: {
      handle: { kind: "petname", id: "river-lantern-117" },
      metaclass: "PartDefinition",
      name: "ThermalController",
      shortName: null,
      qualifiedName: "ThermalControl::ThermalController",
    },
    facet: { kind: "type", isAbstract: false, specializes: [IMPLICIT_PART] },
    owner: PACKAGE,
    visibility: "public",
    location: { file: "model/ThermalControl.sysml", span: { start: 212, end: 598 } },
    documentation: ["Closed-loop regulator for panel temperature."],
    ownedFeatures: [
      feature({
        id: "birch-ember-130",
        metaclass: "AttributeUsage",
        name: "setpoint",
        type: TEMPERATURE_VALUE,
      }),
      feature({
        id: "birch-ember-131",
        metaclass: "AttributeUsage",
        name: "tolerance",
        type: REAL,
      }),
      feature({
        id: "birch-ember-132",
        metaclass: "PortUsage",
        name: "cmdIn",
        type: to("jade-anchor-615", "CommandPort"),
      }),
      feature({
        id: "birch-ember-133",
        metaclass: "PortUsage",
        name: "heaterOut",
        type: to("ivory-gate-504", "PowerPort"),
      }),
      // A conjugated port: the target is the conjugated definition, whose name carries the ~.
      feature({
        id: "birch-ember-134",
        metaclass: "PortUsage",
        name: "sensorIn",
        type: {
          kind: "resolved",
          target: { kind: "derived", hash: "5c2e8a17" },
          name: "~TempPort",
        },
      }),
      feature({
        id: "birch-ember-135",
        metaclass: "PerformActionUsage",
        name: "regulate",
        type: to("coral-dune-493", "RegulateTemperature"),
      }),
    ],
    relationships: [
      {
        handle: { kind: "petname", id: "hollow-signal-771" },
        metaclass: "SatisfyRequirementUsage",
        direction: "outgoing",
        other: to("sage-harbor-271", "TemperatureStability"),
        isImplied: false,
      },
      {
        handle: { kind: "derived", hash: "e09b33d4" },
        metaclass: "FeatureTyping",
        direction: "incoming",
        other: to("slate-orchard-406", "controller"),
        isImplied: false,
      },
    ],
    diagnostics: [],
  },
};

const HEATER = {
  kind: "ready",
  data: {
    summary: {
      handle: { kind: "petname", id: "maple-sunrise-314" },
      metaclass: "PartDefinition",
      name: "Heater",
      shortName: null,
      qualifiedName: "ThermalControl::Heater",
    },
    facet: { kind: "type", isAbstract: false, specializes: [IMPLICIT_PART] },
    owner: PACKAGE,
    visibility: "public",
    location: { file: "model/ThermalControl.sysml", span: { start: 602, end: 761 } },
    documentation: ["Resistive film heater."],
    ownedFeatures: [
      feature({
        id: "quill-harbor-240",
        metaclass: "AttributeUsage",
        name: "maxPower",
        type: POWER_VALUE,
      }),
      feature({
        id: "quill-harbor-241",
        metaclass: "AttributeUsage",
        name: "state",
        type: {
          kind: "unresolved",
          written: "HeaterState",
          code: "RES-UNRESOLVED-NAME",
        },
      }),
      feature({
        id: "quill-harbor-242",
        metaclass: "PortUsage",
        name: "pwrIn",
        type: {
          kind: "resolved",
          target: { kind: "derived", hash: "5c2e8a18" },
          name: "~PowerPort",
        },
      }),
    ],
    relationships: [
      {
        handle: { kind: "derived", hash: "e09b33d5" },
        metaclass: "FeatureTyping",
        direction: "incoming",
        other: to("slate-orchard-407", "heater"),
        isImplied: false,
      },
    ],
    diagnostics: [
      {
        code: "RES-UNRESOLVED-NAME",
        severity: "error",
        span: { start: 688, end: 699 },
        message: "cannot resolve type `HeaterState`",
      },
    ],
  },
};

/** A view's parts: identity, name, kind, and the one namespace it exposes. */
type ViewSpec = Readonly<{ id: string; name: string; kind: string; exposes: unknown }>;

/** A view summary exposing one namespace, as every view in the mockup's list does. */
function view({ id, name, kind, exposes }: ViewSpec): unknown {
  return {
    id,
    name,
    kind,
    exposes: [{ kind: "namespace", target: exposes, isRecursive: false }],
  };
}

/** The mockup's workspace, views and two fully described elements. */
export const THERMAL_CONTROL: Fixtures = {
  workspace: {
    kind: "ready",
    data: {
      name: "ThermalControl",
      files: [
        {
          path: "model/ThermalControl.sysml",
          language: "sysml",
          diagnostics: { ...NONE, error: 1 },
        },
        { path: "model/Interfaces.sysml", language: "sysml", diagnostics: NONE },
        { path: "model/Requirements.sysml", language: "sysml", diagnostics: NONE },
        { path: "model/Behavior.sysml", language: "sysml", diagnostics: NONE },
        { path: "library/Units.kerml", language: "kerml", diagnostics: NONE },
        { path: "library/Signals.kerml", language: "kerml", diagnostics: NONE },
        { path: "views/Views.sysml", language: "sysml", diagnostics: NONE },
      ],
    },
  },
  views: {
    kind: "ready",
    data: [
      view({ id: "willow-beacon-551", name: "thermalOverview", kind: "general", exposes: PACKAGE }),
      view({
        id: "birch-signal-612",
        name: "subsystemWiring",
        kind: "interconnection",
        exposes: to("slate-orchard-405", "thermalSubsystem"),
      }),
      view({
        id: "copper-thistle-733",
        name: "regulationFlow",
        kind: "action-flow",
        exposes: to("coral-dune-494", "regulate"),
      }),
      view({
        id: "ember-canyon-844",
        name: "heaterModes",
        kind: "state-transition",
        exposes: to("pine-valley-382", "HeaterModes"),
      }),
      view({
        id: "frost-ledger-955",
        name: "requirementsGrid",
        kind: "grid",
        exposes: to("onyx-prairie-160", "Requirements"),
      }),
    ],
  },
  elements: {
    "river-lantern-117": THERMAL_CONTROLLER,
    "maple-sunrise-314": HEATER,
  },
};
