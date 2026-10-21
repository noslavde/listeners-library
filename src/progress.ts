/**
 * Pure reconciliation for card progress labels. A fresh live observation can be
 * shown directly, while mismatched local/cloud saved values remain a conflict
 * rather than an invented latest position.
 */

import type { CardPositions } from "./useCardPositions";

// A cloud check time is not a listening time. Never claim it proves which
// conflicting position is newer; live playback is the only authoritative value.
/**
 * Prioritizes a current live observation. Otherwise flags saved positions that
 * differ by more than 15 seconds; cloud check timestamps cannot establish which
 * listening position is newer.
 */
export function progressSummary(saved?: CardPositions[string], live?: number) {
  if (live !== undefined)
    return { position: live, source: "Playing here", conflict: false };
  const local = saved?.local?.position,
    cloud = saved?.cloud?.position;
  if (
    local !== undefined &&
    cloud !== undefined &&
    Math.abs(local - cloud) > 15
  )
    return {
      position: undefined,
      source: "Saved positions differ",
      conflict: true,
    };
  if (cloud !== undefined)
    return { position: cloud, source: "Audible", conflict: false };
  return { position: local, source: "Saved on this PC", conflict: false };
}
