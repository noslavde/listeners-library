/**
 * Regression tests covering noteDrafts. Uses synthetic fixtures; these checks
 * do not verify real account sign-in, audio output or mobile synchronization.
 */

import { it, expect } from "vitest";
import { changeDraft, clearDraft } from "./noteDrafts";
it("keeps drafts and their initial timestamps with their own book", () => {
  let d = changeDraft({}, "A", "a thought", 42);
  d = changeDraft(d, "B", "another thought", 80);
  d = changeDraft(d, "A", "edited", 90);
  expect(d.A).toEqual({ text: "edited", position: 42 });
  expect(d.B).toEqual({ text: "another thought", position: 80 });
  expect(clearDraft(d, "A", "old text")).toEqual(d);
  expect(clearDraft(d, "A", "edited").B).toEqual(d.B);
});
