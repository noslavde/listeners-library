/**
 * Chapter dropdown driven by cached book metadata and live seekability.
 * Delegates chapter selection or explicit seeking to its caller instead of
 * assuming metadata alone proves playable content.
 */

import type { Book, SeekRange } from "./model";
/**
 * Keeps the dropdown available from chapter metadata and delegates actual
 * navigation. The caller/backend decides whether the current official player
 * can accept a chapter or seek request.
 */
export function ChapterSelect({
  book,
  position,
  onSeek,
  onChapter,
}: {
  book: Book;
  position: number;
  duration: number;
  available: boolean;
  seekable?: SeekRange[];
  onSeek: (position: number) => void;
  onChapter?: (position: number, title: string) => void;
}) {
  const chapters = book.chapterDetails?.length
    ? book.chapterDetails
    : book.chapters.map((start, i) => ({
        start,
        title: `Chapter ${i + 1}`,
        length: null,
      }));
  const current = chapters.filter((c) => c.start <= position).at(-1)?.start;
  return (
    <label className="chapter-select">
      Chapter{" "}
      <select
        aria-label="Select chapter"
        title="Select a chapter using the official player's chapter controls"
        disabled={!chapters.length}
        value={current ?? ""}
        onChange={(e) => {
          const c = chapters.find((c) => c.start === Number(e.target.value));
          if (c) {
            if (onChapter) onChapter(c.start, c.title);
            else onSeek(c.start);
          }
        }}
      >
        <option value="" disabled>
          {chapters.length ? "Choose chapter" : "No chapter data"}
        </option>
        {chapters.map((c, i) => (
          <option key={`${c.start}-${i}`} value={c.start}>
            {c.title} · {Math.floor(c.start / 3600)}:
            {String(Math.floor(c.start / 60) % 60).padStart(2, "0")}:
            {String(Math.floor(c.start) % 60).padStart(2, "0")}
          </option>
        ))}
      </select>
    </label>
  );
}
