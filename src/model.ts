/**
 * Shared library and playback data contracts plus pure timing helpers.
 * Positions and durations are seconds; browser demo books are fictional
 * fixtures, not account data.
 */

export type Chapter = { title: string; start: number; length: number | null };
export type CloudMetadata = {
  position: number | null;
  chapters: number[];
  chapter_details: Chapter[];
  duration: number | null;
  position_source: string;
  warnings: string[];
};
export type Book = {
  asin: string;
  title: string;
  author: string;
  series: string;
  volume: number;
  duration: number;
  chapters: number[];
  chapterDetails?: Chapter[];
  coverUrl?: string;
};
export type SeekRange = { start: number; end: number };
export type Snapshot = {
  position: number;
  duration: number;
  speed: number;
  paused: boolean;
  available: boolean;
  sample?: boolean;
  seekable?: SeekRange[];
  buffered?: SeekRange[];
  volume?: number;
};
export const demoBooks: Book[] = [
  {
    asin: "DEMO000001",
    title: "The Observatory",
    author: "Demo author",
    series: "Northern Skies",
    volume: 1,
    duration: 7200,
    chapters: [0, 1800, 3600, 5400],
  },
  {
    asin: "DEMO000002",
    title: "Beyond the Horizon",
    author: "Demo author",
    series: "Northern Skies",
    volume: 2,
    duration: 10800,
    chapters: [0, 2700, 5400, 8100],
  },
];
/**
 * Returns percentage through the current chapter using book-level seconds; this
 * is a display calculation, not a seekability check.
 */
export function chapterProgress(book: Book, p: number) {
  const start = book.chapters.filter((x) => x <= p).at(-1) ?? 0;
  const end = book.chapters.find((x) => x > p) ?? book.duration;
  return end > start ? Math.round(((p - start) / (end - start)) * 100) : 0;
}
/**
 * Returns book seconds remaining after accounting for playback speed, suitable
 * for a finish-time estimate.
 */
export function remaining(s: Snapshot) {
  return Math.max(0, s.duration - s.position) / s.speed;
}
