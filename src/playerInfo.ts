import type { Book, Snapshot, Chapter } from "./model";
export function orderedChapters(book: Book | undefined): Chapter[] {
  if (!book) return [];
  return (
    book.chapterDetails?.length
      ? book.chapterDetails
      : book.chapters.map((start, i) => ({
          start,
          title: `Chapter ${i + 1}`,
          length: null,
        }))
  )
    .filter((c) => Number.isFinite(c.start) && c.start >= 0)
    .slice()
    .sort((a, b) => a.start - b.start)
    .filter((c, i, all) => i === 0 || c.start !== all[i - 1].start);
}
export function adjacentChapters(book: Book | undefined, position: number) {
  const chapters = orderedChapters(book);
  const index = chapters.reduce(
    (last, c, i) => (c.start <= position ? i : last),
    -1,
  );
  return { previous: chapters[index - 1], next: chapters[index + 1] };
}
export function chapterInfo(book: Book | undefined, s: Snapshot) {
  if (!book || !s.available || s.duration <= 0) return null;
  const chapters = orderedChapters(book).filter((c) => c.start < s.duration);
  const index = chapters.reduce(
    (last, c, i) => (c.start <= s.position ? i : last),
    -1,
  );
  if (index < 0) return null;
  const c = chapters[index],
    end = Math.min(
      s.duration,
      chapters[index + 1]?.start ??
        (c.length && c.length > 0 ? c.start + c.length : s.duration),
    );
  const position = Math.max(0, Math.min(end - c.start, s.position - c.start)),
    duration = end - c.start;
  return {
    title: c.title,
    start: c.start,
    end,
    position,
    duration,
    remaining: Math.max(0, end - s.position) / Math.max(0.5, s.speed),
    percent: duration > 0 ? Math.round((position / duration) * 100) : 0,
  };
}
export function finishTime(s: Snapshot, now = Date.now()) {
  return (
    now + (Math.max(0, s.duration - s.position) / Math.max(0.5, s.speed)) * 1000
  );
}
