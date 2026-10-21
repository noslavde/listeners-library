/**
 * Pure helpers for the earlier note-draft model. Preserve a draft's original
 * timestamp while typing and avoid clearing newer text when a slow save
 * finishes.
 */

export type Draft = { text: string; position: number };
export type Drafts = Record<string, Draft>;
/**
 * Retains the first timestamp while text is being edited so playback movement
 * cannot silently move an unfinished note.
 */
export function changeDraft(
  drafts: Drafts,
  asin: string,
  text: string,
  position: number,
): Drafts {
  return {
    ...drafts,
    [asin]: {
      text,
      position: drafts[asin]?.text ? drafts[asin].position : position,
    },
  };
}
/**
 * Removes only the draft text that actually finished saving; leaves newer
 * typing intact.
 */
export function clearDraft(
  drafts: Drafts,
  asin: string,
  savedText: string,
): Drafts {
  // A slow save must not clear new text typed while the write was pending.
  if (drafts[asin]?.text !== savedText) return drafts;
  const next = { ...drafts };
  delete next[asin];
  return next;
}
