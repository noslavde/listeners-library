/**
 * Pure sorting and continue-listening eligibility rules. Returns copied arrays
 * for display and uses verified listening activity rather than cloud refresh
 * time to rank recent listening.
 */

import type { Book } from "./model";
import type { CardPositions } from "./useCardPositions";
import { progressSummary } from "./progress";
export type Sort =
  | "title"
  | "author"
  | "series"
  | "recent"
  | "shortest"
  | "longest"
  | "progress"
  | "rating";
export type Activity = Record<
  string,
  {
    listenedSeconds: number;
    lastListened: number;
    position: number;
    duration: number;
    hidden: boolean;
  }
>;
const compare = (a: string, b: string) =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });
/**
 * Returns a sorted copy with stable title/ASIN tie-breakers. Rating order is
 * applied by the caller because local reviews are a separate data source.
 */
export function sortedBooks(
  books: Book[],
  sort: Sort,
  activity: Activity,
  positions: CardPositions,
) {
  const progress = (b: Book) => {
    const saved = positions[b.asin],
      summary = progressSummary(saved);
    return b.duration > 0 && !summary.conflict
      ? (summary.position ?? activity[b.asin]?.position ?? 0) / b.duration
      : -1;
  };
  return books.slice().sort((a, b) => {
    let order = 0;
    switch (sort) {
      case "author":
        order = compare(a.author, b.author);
        break;
      case "series":
        order =
          Number(!a.series) - Number(!b.series) ||
          compare(a.series, b.series) ||
          a.volume - b.volume;
        break;
      case "recent":
        order =
          (activity[b.asin]?.lastListened ?? 0) -
          (activity[a.asin]?.lastListened ?? 0);
        break;
      case "shortest":
        order = (a.duration || Infinity) - (b.duration || Infinity);
        break;
      case "longest":
        order = (b.duration || 0) - (a.duration || 0);
        break;
      case "progress":
        order = progress(b) - progress(a);
        break;
    }
    return order || compare(a.title, b.title) || compare(a.asin, b.asin);
  });
}
/**
 * Requires meaningful credited listening, recent interaction and incomplete
 * progress. Caps the shelf at six titles; seeking or merely checking cloud
 * state does not earn credit.
 */
export function continueListening(
  books: Book[],
  activity: Activity,
  now = Date.now() / 1000,
) {
  return books
    .filter((b) => {
      const a = activity[b.asin],
        duration = b.duration || a?.duration;
      if (!a || a.hidden || !duration || duration <= 0) return false;
      const threshold = Math.max(30, Math.min(300, duration * 0.05));
      return (
        a.listenedSeconds >= threshold &&
        a.lastListened >= now - 30 * 86400 &&
        a.lastListened <= now + 60 &&
        a.position > 0 &&
        a.position / duration < 0.98
      );
    })
    .sort(
      (a, b) =>
        activity[b.asin].lastListened - activity[a.asin].lastListened ||
        compare(a.title, b.title),
    )
    .slice(0, 6);
}
