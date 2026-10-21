/**
 * Injected app-only media volume controller for official playback. Uses normal
 * media attenuation (0..100%) without creating a Web Audio graph; explicit
 * volume choices alone may unmute the media.
 */

// App-only volume on the official media element. No processing or system mixer.
(() => {
  if (
    location.protocol !== "https:" ||
    !["www.audible.com", "audible.com"].includes(location.hostname) ||
    location.pathname !== "/webplayer"
  )
    return;
  const requested = new WeakMap();
  const status = (m) => ({ volume: m.muted ? 0 : (m.volume ?? 1) });
  const apply = (m, value, explicit = false) => {
    if (!Number.isFinite(value) || value < 0 || value > 1) return status(m);
    if (requested.get(m) === value && !explicit) return status(m);
    requested.set(m, value);
    if (explicit && value > 0) m.muted = false;
    m.volume = value;
    return status(m);
  };
  window.__audibleVolume = { apply, status };
})();
