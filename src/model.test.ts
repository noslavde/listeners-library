/**
 * Regression tests covering player calculations. Uses synthetic fixtures; these
 * checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { describe, it, expect } from "vitest";
import { chapterProgress, remaining, demoBooks } from "./model";
describe("player calculations", () => {
  it("uses speed for remaining time", () =>
    expect(
      remaining({
        position: 120,
        duration: 720,
        speed: 2,
        paused: false,
        available: true,
      }),
    ).toBe(300));
  it("resets percentage at chapter boundaries", () => {
    expect(chapterProgress(demoBooks[0], 900)).toBe(50);
    expect(chapterProgress(demoBooks[0], 1800)).toBe(0);
  });
});
