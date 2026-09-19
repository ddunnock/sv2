// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The composition root, and the only module that does work at load time
 * (STD-004-TS §2.2).
 *
 * It does three things in this order and nothing else: configure Zod for the
 * Tauri content-security policy, install the global failure handlers, then
 * create the React root and render the shell.
 *
 * Every other module defines and does not execute (§3.3). A module that needs a
 * service receives it here, through a parameter or React context, and never
 * constructs one at import time — which is why the reporter is passed into
 * `installGlobalHandlers` rather than imported by it.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { z } from "zod";
import { installGlobalHandlers, reportToConsole } from "@/diagnostics/handlers";
import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/fixture-thermal-control";
import { createModelQueries } from "@/ipc/model-queries";
import { Shell } from "@/shell/Shell";

// 1. Configure Zod for the Tauri CSP, before anything can parse (§4.3 rule 6).
//    Zod's compiled fast path builds functions with `new Function`, and the
//    content-security policy allows no `unsafe-eval`. §2.1's row for this file
//    does not list `zod`; rule 6 names this file, and the row is the defect.
z.config({ jitless: true });

// 2. Install the global failure handlers, before the first render can fail.
installGlobalHandlers(reportToConsole);

// 3. Create the React root and render the shell, handing it its services
//    (§2.2). Answers come from the mockup's sample model until the Tauri client
//    exists (Phase 6); `provenance` makes the status bar say so. §2.1's row for
//    this file lists neither `ipc` nor `zod`, but §2.2 requires the services to
//    be built here, so that row is the defect.
const services = {
  queries: createModelQueries(fixtureTransport(THERMAL_CONTROL), "fixture"),
  report: reportToConsole,
} as const;

const container = document.getElementById("root");
if (container === null) {
  throw new Error("index.html has no #root element to mount into");
}

createRoot(container).render(
  <StrictMode>
    <Shell services={services} />
  </StrictMode>,
);
