/**
 * Regression tests covering Continue listening policy; library sorting. Uses
 * synthetic fixtures; these checks do not verify real account sign-in, audio
 * output or mobile synchronization.
 */

import { describe, it, expect } from "vitest";
import { continueListening, sortedBooks, type Activity } from "./libraryView";
import type { Book } from "./model";
const now = 1800000000;
function book(asin: string, title = asin, duration = 36000): Book {
  return {
    asin,
    title,
    duration,
    author: "Author",
    series: "",
    volume: 0,
    chapters: [],
  };
}
function activity(overrides: Partial<Activity[string]> = {}): Activity[string] {
  return {
    listenedSeconds: 300,
    lastListened: now,
    position: 2000,
    duration: 36000,
    hidden: false,
    ...overrides,
  };
}
describe("Continue listening policy", () => {
  it("requires meaningful real listening rather than merely a saved position", () => {
    const b = book("A");
    expect(
      continueListening(
        [b],
        { A: activity({ listenedSeconds: 20, position: 10000 }) },
        now,
      ),
    ).toEqual([]);
    expect(continueListening([b], { A: activity() }, now)).toEqual([b]);
    expect(continueListening([b], {}, now)).toEqual([]);
  });
  it("lets old and nearly finished books fall off without deleting history", () => {
    const b = book("A");
    for (const patch of [
      { lastListened: now - 31 * 86400 },
      { position: 36000 * 0.98 },
      { hidden: true },
      { position: 0 },
    ])
      expect(continueListening([b], { A: activity(patch) }, now)).toEqual([]);
  });
  it("uses a smaller threshold for short titles with a 30-second minimum", () => {
    const b = book("A", "Short", 600);
    expect(
      continueListening(
        [b],
        { A: activity({ duration: 600, position: 35, listenedSeconds: 29 }) },
        now,
      ),
    ).toEqual([]);
    expect(
      continueListening(
        [b],
        { A: activity({ duration: 600, position: 35, listenedSeconds: 30 }) },
        now,
      ),
    ).toEqual([b]);
  });
  it("orders by actual listening recency and limits the shelf to six", () => {
    const books = Array.from({ length: 8 }, (_, i) => book(String(i))),
      history = Object.fromEntries(
        books.map((b, i) => [b.asin, activity({ lastListened: now - i * 60 })]),
      );
    expect(
      continueListening(books.reverse(), history, now).map((b) => b.asin),
    ).toEqual(["0", "1", "2", "3", "4", "5"]);
  });
  it("can use a known playback duration when library runtime is missing", () => {
    const b = book("A", "No runtime", 0);
    expect(continueListening([b], { A: activity() }, now)).toEqual([b]);
  });
});
describe("library sorting", () => {
  const a = {
      ...book("A", "The 10th Book", 7200),
      series: "Series",
      volume: 10,
      author: "Zed",
    },
    b = {
      ...book("B", "The 2nd Book", 3600),
      series: "Series",
      volume: 2,
      author: "Amy",
    },
    c = book("C", "Standalone", 0);
  it("sorts titles naturally without mutating the library", () => {
    const rows = [a, b, c];
    expect(sortedBooks(rows, "title", {}, {}).map((b) => b.asin)).toEqual([
      "C",
      "B",
      "A",
    ]);
    expect(rows).toEqual([a, b, c]);
  });
  it("keeps series in volume order and standalone titles after series", () => {
    expect(sortedBooks([c, a, b], "series", {}, {}).map((b) => b.asin)).toEqual(
      ["B", "A", "C"],
    );
  });
  it("places unknown runtime last in both length orders", () => {
    expect(
      sortedBooks([c, a, b], "shortest", {}, {}).map((b) => b.asin),
    ).toEqual(["B", "A", "C"]);
    expect(
      sortedBooks([c, a, b], "longest", {}, {}).map((b) => b.asin),
    ).toEqual(["A", "B", "C"]);
  });
  it("uses listening activity for recency rather than a cloud check timestamp", () => {
    expect(
      sortedBooks(
        [a, b],
        "recent",
        { A: activity({ lastListened: now - 10 }), B: activity() },
        { A: { cloud: { position: 500, updated: now + 200 } } },
      ).map((b) => b.asin),
    ).toEqual(["B", "A"]);
  });
  it("sorts author and progress with stable title ties", () => {
    expect(sortedBooks([a, b], "author", {}, {}).map((b) => b.asin)).toEqual([
      "B",
      "A",
    ]);
    expect(
      sortedBooks(
        [a, b],
        "progress",
        {},
        {
          A: { local: { position: 7000, updated: now } },
          B: { cloud: { position: 100, updated: now } },
        },
      ).map((b) => b.asin),
    ).toEqual(["A", "B"]);
  });
});
