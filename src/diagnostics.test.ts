/**
 * Regression tests covering private error reporting. Uses synthetic fixtures;
 * these checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("./playback", () => ({ desktop: true }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));
import { invoke } from "@tauri-apps/api/core";
import { installErrorReporting, reportUiError } from "./diagnostics";
afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("private error reporting", () => {
  it("never forwards event content or rejection details", () => {
    const target = new EventTarget();
    vi.stubGlobal("window", target);
    const stop = installErrorReporting();
    for (const type of ["error", "unhandledrejection"]) {
      const event = new Event(type);
      Object.assign(event, {
        message: "account@example.test",
        reason: "https://secret.test/?token=private",
        error: new Error("private note"),
      });
      target.dispatchEvent(event);
    }
    reportUiError("react-error");
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ["record_ui_error", { code: "script-error" }],
      ["record_ui_error", { code: "promise-rejected" }],
      ["record_ui_error", { code: "react-error" }],
    ]);
    stop();
    target.dispatchEvent(new Event("error"));
    expect(invoke).toHaveBeenCalledTimes(3);
  });
  it("does not create another unhandled rejection when logging fails", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("unavailable"));
    reportUiError("react-error");
    await Promise.resolve();
    expect(invoke).toHaveBeenCalledOnce();
  });
});
