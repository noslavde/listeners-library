/**
 * Regression tests covering direct playback controls. Uses synthetic fixtures;
 * these checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { describe, it, expect } from "vitest";
import {
  dispatchDirectCommand,
  registerDirectCommands,
  playFailure,
} from "./directCommands";

describe("direct playback controls", () => {
  it("dispatches in the click call stack and falls back after teardown", () => {
    const calls: string[] = [];
    const stop = registerDirectCommands((action) => calls.push(action));
    expect(dispatchDirectCommand("play", 0)).toBe(true);
    expect(calls).toEqual(["play"]);
    stop();
    expect(dispatchDirectCommand("play", 0)).toBe(false);
  });
  it("does not let an old engine unregister the current engine", () => {
    const stopOld = registerDirectCommands(() => {});
    const calls: string[] = [];
    const stopCurrent = registerDirectCommands((action) => calls.push(action));
    stopOld();
    expect(dispatchDirectCommand("pause", 0)).toBe(true);
    expect(calls).toEqual(["pause"]);
    stopCurrent();
  });
  it("distinguishes permission failures from format and interrupted playback without disclosing raw errors", () => {
    expect(playFailure({ name: "NotAllowedError" })).toContain(
      "permission denied",
    );
    expect(playFailure({ name: "NotSupportedError" })).toContain(
      "audio format",
    );
    expect(playFailure({ name: "AbortError" })).toContain("interrupted");
    expect(
      playFailure({
        name: "UnknownError",
        message: "https://private.example/?token=secret",
      }),
    ).not.toContain("secret");
  });
});
