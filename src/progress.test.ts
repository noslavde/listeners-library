/**
 * Regression tests covering saved position conflicts. Uses synthetic fixtures;
 * these checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { describe, it, expect } from "vitest";
import { progressSummary } from "./progress";
describe("saved position conflicts", () => {
  it("does not mistake a cloud check timestamp for newer listening", () => {
    expect(
      progressSummary({
        local: { position: 90, updated: 100 },
        cloud: { position: 200, updated: 999 },
      }),
    ).toEqual({
      position: undefined,
      source: "Saved positions differ",
      conflict: true,
    });
  });
  it("trusts current playback over saved conflicts", () => {
    expect(
      progressSummary(
        {
          local: { position: 90, updated: 100 },
          cloud: { position: 200, updated: 999 },
        },
        125,
      ).position,
    ).toBe(125);
  });
  it("uses account position when saved positions agree within one skip", () => {
    expect(
      progressSummary({
        local: { position: 90, updated: 100 },
        cloud: { position: 100, updated: 999 },
      }).position,
    ).toBe(100);
  });
});
