import type { CardPositions } from "./useCardPositions";

// A cloud check time is not a listening time. Never claim it proves which
// conflicting position is newer; live playback is the only authoritative value.
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
