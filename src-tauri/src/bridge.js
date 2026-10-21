/**
 * Injected official-player control/telemetry bridge for allowlisted Audible
 * pages. Reads media state and enforces resume/clip/chapter guards; it has no
 * general Tauri command access and does not handle account secrets.
 */

// No Tauri IPC is granted to this remote page. This bridge only reports media state.
(() => {
  if (
    location.protocol !== "https:" ||
    !["www.audible.com", "audible.com"].includes(location.hostname) ||
    location.pathname !== "/webplayer"
  )
    return;
  let media,
    last = 0,
    recovery = 0,
    pausedAt = 0,
    rewind = 5,
    pauseChapter = false,
    chapter = "",
    intentUntil = 0,
    chapterStarts = [],
    configuredMedia,
    configuredSpeed,
    resumeHold = true,
    clipRange = null;
  // Start held so initial site autoplay cannot replace a saved PC checkpoint
  // before the native startup/history guard has reviewed the service position.
  document.addEventListener?.(
    "play",
    (e) => {
      if (resumeHold && e.target?.pause) e.target.pause();
    },
    true,
  );
  document.addEventListener?.(
    "seeking",
    (e) => {
      if (
        clipRange &&
        e.target === clipRange.media &&
        (e.target.currentTime < clipRange.start ||
          e.target.currentTime > clipRange.end)
      )
        clipRange = null;
    },
    true,
  );
  const ranges = (r) => {
    const result = [];
    for (let i = 0; i < Math.min(r.length, 64); i++) {
      const start = r.start(i),
        end = r.end(i);
      if (
        Number.isFinite(start) &&
        Number.isFinite(end) &&
        start >= 0 &&
        end >= start
      )
        result.push({ start, end });
    }
    return result;
  };
  const find = () => {
    const elements = [...document.querySelectorAll("audio,video")];
    const playing = elements.find(
      (m) => !m.paused && !m.ended && m.readyState > 0,
    );
    if (playing) return playing;
    if (media && elements.includes(media) && media.readyState > 0) return media;
    return (
      elements
        .filter((m) => Number.isFinite(m.duration) && m.duration > 0)
        .sort((a, b) => b.duration - a.duration)[0] ?? elements[0]
    );
  };
  const error = (code) => {
    location.href = "audible-control://error?code=" + encodeURIComponent(code);
  };
  const seek = (m, value) => {
    if (
      !Number.isFinite(value) ||
      !ranges(m.seekable).some((r) => value >= r.start && value <= r.end)
    ) {
      error("unsupported_seek");
      return false;
    }
    try {
      m.currentTime = value;
      return true;
    } catch {
      error("unsupported_seek");
      return false;
    }
  };
  const normalize = (s) =>
    (s ?? "").replace(/\s+/g, " ").trim().toLocaleLowerCase();
  const visible = (e) =>
    e.getClientRects().length > 0 && e.getAttribute("aria-hidden") !== "true";
  const chapterMatch = (title) => {
    const matches = [];
    for (const scope of document.querySelectorAll(
      '[role="dialog"],[aria-label*="chapter" i],[data-testid*="chapter" i],[class*="chapter" i],[id*="chapter" i]',
    )) {
      if (!visible(scope)) continue;
      for (const e of scope.querySelectorAll(
        'button,[role="button"],[role="option"],a,li,span,div',
      )) {
        if (
          !visible(e) ||
          e.disabled ||
          e.getAttribute("aria-disabled") === "true"
        )
          continue;
        if (
          normalize(e.textContent) !== normalize(title) &&
          normalize(e.getAttribute("aria-label")) !== normalize(title)
        )
          continue;
        const target =
          e.closest('button,[role="button"],[role="option"],a,li') ?? e;
        if (!matches.includes(target)) matches.push(target);
      }
    }
    return matches;
  };
  const chooseChapter = ({ position, title }) => {
    intentUntil = Date.now() + 5000;
    const tryClick = () => {
      const matches = chapterMatch(title);
      if (matches.length === 1) {
        matches[0].click();
        return true;
      }
      if (matches.length > 1) {
        error("chapter_ambiguous");
        return true;
      }
      return false;
    };
    if (tryClick()) return;
    const toggle = [
      ...document.querySelectorAll('button,[role="button"]'),
    ].filter(
      (e) =>
        visible(e) &&
        !e.disabled &&
        ["chapters", "chapter list", "table of contents"].includes(
          normalize(e.getAttribute("aria-label") || e.textContent),
        ),
    );
    if (toggle.length === 1) {
      toggle[0].click();
      let attempts = 0;
      const retry = () => {
        if (tryClick()) return;
        if (++attempts < 5) setTimeout(retry, 150);
        else error("chapter_unavailable");
      };
      setTimeout(retry, 150);
      return;
    }
    const m = find();
    if (
      m &&
      ranges(m.seekable).some((r) => position >= r.start && position <= r.end)
    ) {
      seek(m, position);
      return;
    }
    error("chapter_unavailable");
  };
  window.__audibleAdapter = {
    hold(value) {
      const next = !!value;
      if (resumeHold && !next) pausedAt = 0;
      resumeHold = next;
      if (next) find()?.pause();
    },
    chapter: chooseChapter,
    configure(config) {
      const m = find();
      if (!m) return;
      if (
        Number.isFinite(config.speed) &&
        config.speed >= 0.5 &&
        config.speed <= 3 &&
        (m !== configuredMedia || configuredSpeed !== config.speed)
      ) {
        m.playbackRate = config.speed;
        configuredSpeed = config.speed;
        configuredMedia = m;
      }
      rewind = Math.max(0, Math.min(30, config.rewind));
      pauseChapter = !!config.pauseChapter;
      chapterStarts = Array.isArray(config.chapters)
        ? config.chapters
            .filter((v) => Number.isFinite(v) && v >= 0)
            .sort((a, b) => a - b)
        : [];
      window.__audibleVolume?.apply(
        m,
        Number.isFinite(config.volume) ? config.volume : 1,
      );
    },
    command(action, value) {
      const m = find();
      if (!m) return;
      if (["seek", "recover"].includes(action)) clipRange = null;
      intentUntil = Date.now() + 2000;
      if (action === "play" && !resumeHold) {
        if (pausedAt && Date.now() - pausedAt > 60000)
          seek(m, Math.max(0, m.currentTime - rewind));
        m.play().catch(() => error("play_blocked"));
      }
      if (action === "pause") m.pause();
      if (action === "seek") seek(m, value);
      if (action === "volume") window.__audibleVolume?.apply(m, value, true);
      if (action === "speed") m.playbackRate = value;
      if (action === "pauseChapter") pauseChapter = !!value;
      if (action === "rewind") rewind = Math.max(0, Math.min(30, value));
      if (action === "recover") seek(m, recovery);
    },
  };
  const stopClip = (m) => {
    if (!clipRange) return;
    if (m !== clipRange.media || m.currentTime < clipRange.start - 1) {
      clipRange = null;
      return;
    }
    if (m.currentTime >= clipRange.end) {
      const end = clipRange.end;
      clipRange = null;
      m.pause();
      seek(m, end);
    }
  };
  const clipMedia = new WeakSet();
  window.__audibleAdapter.clip = (start, end) => {
    const m = find();
    if (
      !m ||
      resumeHold ||
      !Number.isFinite(start) ||
      !Number.isFinite(end) ||
      end <= start ||
      end - start > 300 ||
      !ranges(m.seekable).some((r) => start >= r.start && end <= r.end)
    ) {
      error("unsupported_seek");
      return;
    }
    clipRange = null;
    if (!seek(m, start)) return;
    pausedAt = 0;
    intentUntil = Date.now() + 2000;
    clipRange = { media: m, start, end };
    if (!clipMedia.has(m)) {
      clipMedia.add(m);
      m.addEventListener("timeupdate", () => stopClip(m));
    }
    m.play().catch(() => {
      clipRange = null;
      error("play_blocked");
    });
  };
  const selectChapter = window.__audibleAdapter.chapter;
  window.__audibleAdapter.chapter = (value) => {
    clipRange = null;
    selectChapter(value);
  };
  setInterval(() => {
    const m = find();
    if (m !== media) {
      media = m;
      last = m?.currentTime ?? 0;
      recovery = last;
      if (m) m.addEventListener("pause", () => (pausedAt = Date.now()));
    }
    if (m) {
      stopClip(m);
      if (resumeHold && !m.paused) m.pause();
      const natural =
        Date.now() > intentUntil &&
        !m.seeking &&
        m.currentTime >= last &&
        m.currentTime - last <= 20;
      if (
        !clipRange &&
        pauseChapter &&
        natural &&
        chapterStarts.some((start) => start > last && start <= m.currentTime)
      )
        m.pause();
      if (Date.now() > intentUntil && Math.abs(m.currentTime - last) > 20)
        recovery = last;
      last = m.currentTime;
      // Keep the old label fallback only when authoritative offsets are unavailable.
      const next =
        document.querySelector('[data-testid="chapter-title"]')?.textContent ??
        "";
      if (
        !chapterStarts.length &&
        next &&
        chapter &&
        next !== chapter &&
        !clipRange &&
        pauseChapter &&
        natural
      )
        m.pause();
      chapter = next;
    }
    const sample =
      new URL(location.href).searchParams.get("isSample") === "true" ||
      [...document.querySelectorAll("button,a")].some(
        (e) => normalize(e.textContent) === "get this title",
      );
    const config = document.getElementById?.(
      "adbl-cloud-player-container-data",
    );
    const flag = (name) => {
      const value = config?.getAttribute(name);
      return value === "true" ? true : value === "false" ? false : null;
    };
    const volumeState = m
      ? (window.__audibleVolume?.status(m) ?? {
          volume: m.muted ? 0 : (m.volume ?? 1),
        })
      : { volume: 1 };
    const state = {
      sample,
      website_signed_in: flag("data-isUserLoggedIn"),
      website_consumable: flag("data-isConsumable"),
      asin: new URL(location.href).searchParams.get("asin") ?? "",
      position: m && Number.isFinite(m.currentTime) ? m.currentTime : 0,
      duration: m && Number.isFinite(m.duration) ? m.duration : 0,
      speed: m?.playbackRate ?? 1,
      paused: m?.paused ?? true,
      available:
        !sample && !!m && Number.isFinite(m.duration) && m.duration > 0,
      seekable: m ? ranges(m.seekable) : [],
      buffered: m ? ranges(m.buffered) : [],
    };
    location.href =
      "audible-state://snapshot?payload=" +
      encodeURIComponent(JSON.stringify({ ...state, ...volumeState }));
  }, 500);
})();
