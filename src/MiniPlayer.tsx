import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { OfficialAdapter, type Action } from "./playback";
import type { Book, Snapshot } from "./model";
import { ChapterSelect } from "./ChapterSelect";
import { Cover, safeCover } from "./Cover";
import {
  clock,
  PreferenceControls,
  SleepControls,
  speeds,
  useListening,
} from "./listening";
import {
  adjacentChapters,
  chapterInfo,
  finishTime,
  PreviousChapterIntent,
} from "./playerInfo";
import { ShortcutHelp, usePlayerShortcuts } from "./shortcuts";
import { useCurrentBook } from "./CurrentBook";
import { ThemePicker } from "./ThemePicker";
import { VolumeControl } from "./VolumeControl";
import { ListeningHistoryPanel, useListeningHistory } from "./ListeningHistory";
type Panel = "none" | "chapters" | "sleep" | "settings" | "history";
export function MiniPlayer() {
  const adapter = useMemo(() => new OfficialAdapter(), []),
    settings = useListening(),
    current = useCurrentBook();
  const [book, setBook] = useState<Book>(),
    [s, setS] = useState<Snapshot>({
      position: 0,
      duration: 0,
      speed: 1,
      paused: true,
      available: false,
    }),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [panel, setPanel] = useState<Panel>("none");
  const [syncing, setSyncing] = useState(false),
    syncId = useRef(0);
  const previousIntent = useRef(new PreviousChapterIntent());
  const history = useListeningHistory(book),
    reviewShown = useRef("");
  const controlsAvailable = s.available && !history.status?.waiting;
  useEffect(() => {
    const conflict = history.status?.conflict;
    if (!conflict || !book) return;
    const key = `${book.asin}:${conflict.pc.at}:${conflict.audible}`;
    if (key === reviewShown.current) return;
    reviewShown.current = key;
    setPanel("history");
    void invoke("resize_mini_player", { panel: "history" }).catch((e) =>
      setError(String(e)),
    );
  }, [history.status?.conflict, book?.asin]);
  usePlayerShortcuts(setError);
  const run = async (f: () => Promise<unknown>) => {
    setError("");
    setNotice("");
    try {
      await f();
    } catch (e) {
      setError(String(e));
    }
  };
  useEffect(() => {
    let dead = false;
    const stops: (() => void)[] = [];
    const keep = (f: () => void) => {
      if (dead) f();
      else stops.push(f);
    };
    adapter.subscribe(setS).then(keep);
    listen<Book>("player-book", (e) => {
      previousIntent.current.reset();
      reviewShown.current = "";
      setPanel("none");
      syncId.current++;
      setSyncing(false);
      setBook(e.payload);
      setS((v) => ({ ...v, available: false }));
      setNotice("");
      setError("");
    }).then(keep);
    listen<string>("player-error", (e) => setError(e.payload)).then(keep);
    listen("sleep-finished", () =>
      setNotice("Sleep timer finished · playback paused"),
    ).then(keep);
    listen<string>("shortcut-notice", (e) => setNotice(e.payload)).then(keep);
    listen<Book>("player-book-details", (e) => setBook(e.payload)).then(keep);
    invoke<{ book: Book | null; snapshot: Snapshot | null }>("current_player")
      .then((v) => {
        if (dead) return;
        if (v.book) setBook(v.book);
        if (v.snapshot) setS(v.snapshot);
      })
      .catch((e) => {
        if (!dead) setError(String(e));
      });
    return () => {
      dead = true;
      syncId.current++;
      stops.forEach((f) => f());
    };
  }, [adapter]);
  const resync = async () => {
    previousIntent.current.reset();
    if (!book || syncing) return;
    const id = ++syncId.current;
    setSyncing(true);
    setError("");
    setNotice("Checking Audible’s saved position…");
    try {
      const result = await invoke<{ position: number }>("resync_player", {
        asin: book.asin,
      });
      if (id === syncId.current)
        setNotice(`Synced to Audible · ${clock(result.position)}`);
    } catch (e) {
      if (id === syncId.current) {
        setNotice("");
        setError(String(e));
      }
    } finally {
      if (id === syncId.current) setSyncing(false);
    }
  };
  const command = (a: Action, v?: number) => {
    if (a !== "volume") previousIntent.current.reset();
    return run(() => adapter.command(a, v));
  };
  const previousChapter = () => {
    const intent = previousIntent.current.press(book, s.position);
    if (!intent) return;
    void run(async () => {
      if (intent.restart) await adapter.command("seek", intent.chapter.start);
      else await adapter.chapter(intent.chapter.start, intent.chapter.title);
      setNotice(
        intent.restart
          ? "Chapter restarted · press again within 3s for previous"
          : `Previous chapter · ${intent.chapter.title}`,
      );
    });
  };
  const toggle = (next: Panel) =>
    run(async () => {
      if (next === "history") previousIntent.current.reset();
      const value = panel === next ? "none" : next;
      setPanel(value);
      await invoke("resize_mini_player", { panel: value });
    });
  const chapter = chapterInfo(book, s),
    chapterMode = settings.preferences.progressMode === "chapter" && !!chapter;
  const adjacent = adjacentChapters(book, s.position);
  const start = chapterMode ? chapter!.start : 0,
    end = chapterMode ? chapter!.end : s.duration;
  const chapterRemaining =
    settings.sleep?.mode === "chapter" && settings.sleep.chapterEnd !== null
      ? Math.max(0, settings.sleep.chapterEnd - s.position) /
        Math.max(0.5, s.speed)
      : chapter?.remaining;
  const sleepLabel = settings.sleep
    ? settings.sleep.mode === "time"
      ? clock(settings.sleep.remainingSeconds ?? 0)
      : "Chapter end"
    : "Sleep";
  return (
    <div className="mini mini-compact">
      <div className="mini-header">
        {book && (
          <button
            className="mini-cover-button"
            disabled={!safeCover(book.coverUrl)}
            aria-label={`Enlarge cover of ${book.title}`}
            title="Open cover artwork"
            onClick={() => run(() => invoke("open_cover"))}
          >
            <Cover book={book} />
          </button>
        )}
        <div
          className="mini-book mini-drag"
          title="Drag to move"
          onPointerDown={(e) => {
            if (e.button === 0) void run(() => invoke("drag_mini_player"));
          }}
        >
          <div>
            <strong title={book?.title}>
              {book?.title ?? "Choose a book in your library"}
            </strong>
            <p>{book?.author || "Listener’s Library"}</p>
          </div>
        </div>
        <button
          className="mini-return"
          aria-label="Close mini player and return to app"
          title="Back to app; audio keeps playing"
          onClick={() => run(() => invoke("close_mini_player"))}
        >
          ↗
        </button>
      </div>
      <div className="mini-controls">
        <button
          aria-label="Skip back 15 seconds"
          disabled={!controlsAvailable}
          onClick={() => command("seek", Math.max(0, s.position - 15))}
        >
          −15
        </button>
        <button
          className="mini-play primary-action"
          aria-label={s.paused ? "Play" : "Pause"}
          disabled={!controlsAvailable}
          onClick={() => command(s.paused ? "play" : "pause")}
        >
          {s.paused ? "▶" : "Ⅱ"}
        </button>
        <button
          aria-label="Skip forward 15 seconds"
          disabled={!controlsAvailable}
          onClick={() => command("seek", Math.min(s.duration, s.position + 15))}
        >
          +15
        </button>
        <select
          aria-label="Playback speed"
          value={s.speed}
          disabled={!controlsAvailable}
          onChange={(e) => command("speed", Number(e.target.value))}
        >
          {[...new Set([...speeds, s.speed])]
            .sort((a, b) => a - b)
            .map((x) => (
              <option key={x} value={x}>
                {x}×
              </option>
            ))}
        </select>
        <button
          aria-label="Save bookmark"
          title="Save bookmark"
          disabled={!controlsAvailable}
          onClick={() =>
            run(() => invoke("player_shortcut", { action: "bookmark" }))
          }
        >
          ◇
        </button>
      </div>
      <div className="mini-chapter">
        <button
          aria-label="Previous chapter"
          title="Restart chapter; press again within 3 seconds for the previous chapter"
          disabled={!controlsAvailable || !chapter}
          onClick={previousChapter}
        >
          ❮
        </button>
        <strong className="mini-chapter-name" title={chapter?.title}>
          {chapter?.title ?? "Chapter information unavailable"}
        </strong>
        <button
          aria-label="Next chapter"
          title={
            adjacent.next ? `Next: ${adjacent.next.title}` : "Last chapter"
          }
          disabled={!controlsAvailable || !adjacent.next}
          onClick={() => {
            previousIntent.current.reset();
            const c = adjacent.next;
            if (c) void run(() => adapter.chapter(c.start, c.title));
          }}
        >
          ❯
        </button>
      </div>
      <div className="mini-progress">
        <div className="progress-heading">
          <button
            disabled={!settings.ready}
            onClick={() =>
              void settings.update({
                progressMode:
                  settings.preferences.progressMode === "chapter"
                    ? "book"
                    : "chapter",
              })
            }
            title="Switch between chapter and book progress"
          >
            {chapterMode ? "Chapter" : "Book"} ⇄
          </button>
          <span>
            {s.available
              ? `${clock(chapterMode ? chapter!.position : s.position)} / ${clock(chapterMode ? chapter!.duration : s.duration)}`
              : "Waiting for playback"}
          </span>
        </div>
        <input
          type="range"
          aria-label={
            chapterMode ? "Chapter playback position" : "Book playback position"
          }
          min={start}
          max={Math.max(start + 1, end)}
          value={Math.max(start, Math.min(end, s.position))}
          disabled={!controlsAvailable}
          onChange={(e) => command("seek", Number(e.target.value))}
        />
        <div className="mini-timing">
          <span title={chapter?.title}>
            {chapter
              ? `${chapter.percent}% · ${clock(chapter.remaining)} left`
              : "Chapter time unavailable"}
          </span>
          <span>
            {s.available
              ? `${s.paused ? "If resumed" : "Finish"} ${new Date(finishTime(s)).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`
              : "Finish —"}
          </span>
        </div>
      </div>
      <VolumeControl
        compact
        value={settings.preferences.volume}
        onChange={(v) =>
          s.available
            ? adapter.command("volume", v)
            : settings.update({ volume: v })
        }
      />
      <div className="mini-tools">
        <button
          aria-expanded={panel === "chapters"}
          aria-controls="mini-panel"
          title={chapter?.title || "Chapter selection"}
          onClick={() => void toggle("chapters")}
        >
          Chapters
        </button>
        <button
          aria-expanded={panel === "history"}
          aria-controls="mini-panel"
          className={history.status?.conflict ? "timer-active" : ""}
          onClick={() => void toggle("history")}
        >
          History
        </button>
        <button
          disabled={!book || !controlsAvailable || s.sample || syncing}
          aria-busy={syncing}
          title="Fetch Audible’s saved position and move to it, forward or backward. Pause on mobile first and allow it to sync."
          onClick={() => void resync()}
        >
          {syncing ? "Syncing…" : "Sync position"}
        </button>
        <button
          className={settings.sleep ? "timer-active" : ""}
          aria-expanded={panel === "sleep"}
          aria-controls="mini-panel"
          onClick={() => void toggle("sleep")}
        >
          {sleepLabel}
        </button>
        <button
          aria-expanded={panel === "settings"}
          aria-controls="mini-panel"
          onClick={() => void toggle("settings")}
        >
          More
        </button>
      </div>
      {panel !== "none" && (
        <div id="mini-panel" className="mini-panel">
          {panel === "history" && (
            <ListeningHistoryPanel history={history} available={s.available} />
          )}
          {panel === "chapters" && (
            <>
              <p>{chapter?.title ?? "Chapter information unavailable"}</p>
              {book && (
                <ChapterSelect
                  book={book}
                  position={s.position}
                  duration={s.duration}
                  available={s.available}
                  seekable={s.seekable}
                  onChapter={(position, title) => {
                    previousIntent.current.reset();
                    return run(() => adapter.chapter(position, title));
                  }}
                  onSeek={(p) => command("seek", p)}
                />
              )}
            </>
          )}
          {panel === "sleep" && (
            <SleepControls
              settings={settings}
              available={s.available}
              chapterRemaining={chapterRemaining}
            />
          )}{" "}
          {panel === "settings" && (
            <>
              <ThemePicker settings={settings} />
              <PreferenceControls settings={settings} />
              {book && (
                <button
                  disabled={
                    !current.ready || current.busy || current.asin === book.asin
                  }
                  onClick={() =>
                    run(async () => {
                      await current.select(book.asin);
                      setNotice("Set as your current book");
                    })
                  }
                >
                  {current.asin === book.asin
                    ? "✓ Current book"
                    : "Set as current"}
                </button>
              )}
              <button
                disabled={!controlsAvailable}
                onClick={() => command("recover")}
              >
                Recover jump
              </button>
              <ShortcutHelp />
              <button
                onClick={() =>
                  run(() => invoke("official_visibility", { visible: true }))
                }
              >
                Show Audible player
              </button>
              <p className="mini-version">
                Listener’s Library · 0.2.4 · Unofficial beta
              </p>
            </>
          )}
        </div>
      )}
      {(error ||
        settings.error ||
        current.error ||
        notice ||
        history.status?.waiting) && (
        <p className="mini-notice" role="status">
          {error ||
            settings.error ||
            current.error ||
            notice ||
            (history.status?.conflict
              ? "Choose a saved position in History"
              : "Checking saved progress…")}
        </p>
      )}
    </div>
  );
}
