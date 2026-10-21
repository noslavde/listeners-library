/**
 * Saved-passage types and pure timestamp/range validation shared by the editor.
 * Clips contain start/end metadata, not recorded audio; sync state is separate
 * from editable passage content.
 */

export type PassageKind = "bookmark" | "clip" | "note";
export type Passage = {
  kind: PassageKind;
  position: number;
  end_position: number | null;
  text: string;
};
export type Highlight = Passage & {
  id: number;
  asin: string;
  created_ms: number | null;
  sync_state?:
    | "local"
    | "synced"
    | "local-changes"
    | "conflict"
    | "remote-missing"
    | "unverified";
  audible_version?: Passage | null;
};
/**
 * Formats seconds as an editor timestamp, truncating subsecond precision.
 */
export function stamp(seconds: number) {
  const s = Math.floor(Math.max(0, seconds));
  return `${Math.floor(s / 3600)}:${String(Math.floor(s / 60) % 60).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}
/**
 * Parses an explicit hours:minutes:seconds timestamp; undefined signals invalid
 * input instead of silently coercing it to zero.
 */
export function parseStamp(input: string): number | undefined {
  if (!/^\d{1,5}:[0-5]\d:[0-5]\d$/.test(input.trim())) return;
  const [h, m, s] = input.trim().split(":").map(Number);
  const value = h * 3600 + m * 60 + s;
  return value < 31_536_000 ? value : undefined;
}
/**
 * Chooses the preceding 30 seconds within book bounds; callers still validate
 * minimum clip length.
 */
export function excerpt(position: number, duration: number): [number, number] {
  const end = Math.min(Math.floor(position), Math.floor(duration));
  return end > 0
    ? [Math.max(0, end - 30), end]
    : [0, Math.min(30, Math.floor(duration))];
}
/**
 * Copies only editable passage fields, excluding database IDs and sync/conflict
 * metadata from comparisons or save payloads.
 */
export function passageOnly(p: Passage): Passage {
  return {
    kind: p.kind,
    position: p.position,
    end_position: p.end_position,
    text: p.text,
  };
}
/**
 * Returns an actionable validation message, or an empty string for valid
 * passage metadata. Applies UTF-8 byte and finite range limits before native
 * validation.
 */
export function passageError(p: Passage, duration?: number): string {
  if (
    !Number.isFinite(p.position) ||
    p.position < 0 ||
    p.position >= 31_536_000
  )
    return "Enter a start time as hours:minutes:seconds.";
  if (new TextEncoder().encode(p.text).length > 10000)
    return "Keep notes under 10,000 bytes.";
  if (p.kind === "note" && !p.text.trim()) return "Write a note before saving.";
  if (
    p.kind === "clip" &&
    (p.end_position === null ||
      !Number.isFinite(p.end_position) ||
      p.end_position <= p.position ||
      p.end_position - p.position > 300)
  )
    return "Choose a clip between 1 second and 5 minutes long.";
  if (
    duration &&
    (p.position >= duration || (p.end_position ?? p.position) > duration)
  )
    return "Choose times within this book.";
  return "";
}
