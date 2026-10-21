/**
 * Regression tests covering private ratings. Uses synthetic fixtures; these
 * checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { describe, expect, it } from "vitest";
import { ratingOrder, reviewError } from "./reviewModel";
describe("private ratings", () => {
  it("distinguishes unrated from zero and permits review-only entries", () => {
    expect(reviewError({ rating: 0, text: "" })).toBe("");
    expect(reviewError({ rating: 50, text: "" })).toBe("");
    expect(reviewError({ rating: null, text: "A thought" })).toBe("");
    expect(reviewError({ rating: null, text: "  " })).not.toBe("");
  });
  it("rejects invalid tenths and limits UTF-8 bytes", () => {
    for (const rating of [-1, 51, 4.5, NaN, Infinity])
      expect(reviewError({ rating, text: "" })).not.toBe("");
    expect(reviewError({ rating: 45, text: "é".repeat(5000) })).toBe("");
    expect(reviewError({ rating: 45, text: "é".repeat(5001) })).not.toBe("");
  });
  it("sorts zero above unrated while leaving equal ratings stable", () => {
    const entries = [
      undefined,
      { rating: 0, text: "" },
      { rating: 45, text: "" },
      { rating: null, text: "Review" },
    ];
    expect(entries.sort(ratingOrder).map((r) => r?.rating)).toEqual([
      45,
      0,
      null,
      undefined,
    ]);
    expect(
      ratingOrder({ rating: 45, text: "a" }, { rating: 45, text: "b" }),
    ).toBe(0);
  });
});
