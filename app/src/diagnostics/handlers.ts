// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The global failure handlers, installed once by the composition root.
 *
 * `diagnostics` is the ONLY module that reports to the log channel
 * (STD-004-TS §2.1). Everything else returns a `Result` and lets this layer
 * decide what a person sees, so there is one place to change when the channel
 * becomes a Tauri command rather than the console.
 *
 * A failure that reaches `unhandledrejection` or `error` has escaped every
 * boundary that was supposed to type it. That is worth reporting loudly and is
 * why this is installed before the React root exists (§2.2): a failure during
 * the first render must not be the thing that has nowhere to go.
 */

/** Undo the handlers this module installed. Returned so tests can clean up. */
export type Uninstall = () => void;

/**
 * Install the window-level handlers. Called once, by `main.tsx`, and never at
 * module scope (§3.3).
 *
 * `report` is a parameter rather than an import so the composition root chooses
 * the channel; this module does not construct one.
 */
export function installGlobalHandlers(
  report: (what: string, detail: unknown) => void,
): Uninstall {
  const onRejection = (event: PromiseRejectionEvent): void => {
    report("unhandled rejection", event.reason);
  };
  const onError = (event: ErrorEvent): void => {
    report("uncaught error", event.error ?? event.message);
  };

  window.addEventListener("unhandledrejection", onRejection);
  window.addEventListener("error", onError);

  return () => {
    window.removeEventListener("unhandledrejection", onRejection);
    window.removeEventListener("error", onError);
  };
}

/**
 * The channel used until the Rust side owns one.
 *
 * Named rather than inlined so that the one place writing to the console is
 * greppable, and so replacing it with a Tauri command is a change to this
 * function and nothing else.
 */
export function reportToConsole(what: string, detail: unknown): void {
  console.error(`sv2-studio: ${what}`, detail);
}
