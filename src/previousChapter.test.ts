/**
 * Regression tests covering restart and previous chapter intent. Uses synthetic
 * fixtures; these checks do not verify real account sign-in, audio output or
 * mobile synchronization.
 */

import { describe, it, expect } from "vitest";
import { PreviousChapterIntent } from "./playerInfo";
import { demoBooks } from "./model";
const book = demoBooks[0];
describe("restart and previous chapter intent", () => {
  it("restarts first, then steps backward on quick repeats despite delayed telemetry", () => {
    const intent = new PreviousChapterIntent();
    expect(intent.press(book, 4000, 100)).toMatchObject({
      chapter: { start: 3600 },
      restart: true,
    });
    expect(intent.press(book, 4000, 200)).toMatchObject({
      chapter: { start: 1800 },
      restart: false,
    });
    expect(intent.press(book, 4000, 300)).toMatchObject({
      chapter: { start: 0 },
      restart: false,
    });
    expect(intent.press(book, 4000, 400)?.chapter.start).toBe(0);
  });
  it("returns to restart behavior after three seconds or another control", () => {
    const intent = new PreviousChapterIntent();
    intent.press(book, 4000, 100);
    expect(intent.press(book, 4000, 3101)).toMatchObject({
      chapter: { start: 3600 },
      restart: true,
    });
    intent.reset();
    expect(intent.press(book, 2000, 3200)).toMatchObject({
      chapter: { start: 1800 },
      restart: true,
    });
  });
  it("restarts the first chapter and never goes below zero", () => {
    const intent = new PreviousChapterIntent();
    expect(intent.press(book, 100, 100)?.chapter.start).toBe(0);
    expect(intent.press(book, 0, 200)?.chapter.start).toBe(0);
  });
  it("does not carry a repeat into a different book or a backwards clock", () => {
    const intent = new PreviousChapterIntent();
    intent.press(book, 4000, 1000);
    expect(intent.press(demoBooks[1], 3000, 1100)).toMatchObject({
      chapter: { start: 2700 },
      restart: true,
    });
    expect(intent.press(demoBooks[1], 3000, 900)).toMatchObject({
      chapter: { start: 2700 },
      restart: true,
    });
  });
});
