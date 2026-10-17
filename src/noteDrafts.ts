export type Draft = { text: string; position: number };
export type Drafts = Record<string, Draft>;
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
