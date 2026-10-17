import { macExperimental, protectionName } from "./platform";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  demoBooks,
  chapterProgress,
  remaining,
  type Book,
  type Snapshot,
} from "./model";
import { desktop, DemoAdapter, OfficialAdapter, type Action } from "./playback";
import "./style.css";
import { MiniPlayer } from "./MiniPlayer";
import { MetadataPanel } from "./MetadataPanel";
import { ListeningControls, useListening } from "./listening";
import { ContinueShelf, useLibraryActivity } from "./LibraryShelf";
import { sortedBooks, type Sort } from "./libraryView";
import { ShortcutHelp, usePlayerShortcuts } from "./shortcuts";
import { Cover } from "./Cover";
import { CurrentBook, useCurrentBook } from "./CurrentBook";
import { ThemeManager, ThemePicker } from "./ThemePicker";
import { applyTheme, cachedTheme } from "./themes";
import { ExploreNav } from "./ExploreNav";
import { VolumeControl } from "./VolumeControl";
import { PrivacyPanel } from "./PrivacyPanel";

import { ResumePanel } from "./ResumePanel";
import { ChapterSelect } from "./ChapterSelect";
import { CardProgress } from "./CardProgress";
import { useCardPositions } from "./useCardPositions";
import { NotesPanel } from "./NotesPanel";
import { ErrorBoundary } from "./ErrorBoundary";
function App() {
  const libraryRevision = useRef(0),
    accountEpoch = useRef(0),
    accountConnected = useRef(!desktop);
  const [connected, setConnected] = useState(!desktop);
  const [live, setLive] = useState(false),
    [books, setBooks] = useState<Book[]>(desktop ? [] : demoBooks),
    [active, setActive] = useState<Book>(),
    [query, setQuery] = useState(""),
    [series, setSeries] = useState(""),
    [status, setStatus] = useState(
      desktop
        ? "Choose a book to open the mini player"
        : "Browser demo · simulated playback",
    );
  const [s, setS] = useState<Snapshot>({
    position: 0,
    duration: 0,
    speed: 1,
    paused: true,
    available: false,
  });
  const cardProgress = useCardPositions(books.map((b) => b.asin).join(","));
  const activity = useLibraryActivity(),
    settings = useListening(),
    [demoSort, setDemoSort] = useState<Sort>("title");
  const current = useCurrentBook(),
    currentTitle = books.find((b) => b.asin === current.asin);
  usePlayerShortcuts(setStatus);
  const accessibleBooks = connected ? books : [];
  const visibleBooks = sortedBooks(
    accessibleBooks.filter(
      (b) =>
        (!series || b.series === series) &&
        `${b.title} ${b.author}`.toLowerCase().includes(query.toLowerCase()),
    ),
    desktop ? settings.preferences.librarySort : demoSort,
    activity,
    cardProgress.positions,
  );
  const adapter = useMemo(
    () => (live ? new OfficialAdapter() : new DemoAdapter()),
    [live],
  );
  const run = async (task: () => Promise<unknown>) => {
    try {
      await task();
    } catch (e) {
      setStatus(String(e));
    }
  };
  useEffect(() => {
    let dispose: (() => void) | undefined,
      cancelled = false;
    adapter
      .subscribe((value) => {
        if (!desktop || accountConnected.current) setS(value);
      })
      .then((f) => {
        if (cancelled) f();
        else dispose = f;
      })
      .catch(() => {
        if (!cancelled)
          setStatus(
            "Playback controls could not connect. Close and reopen the app.",
          );
      });
    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [adapter]);
  const accountChanged = useCallback((value: boolean) => {
    const previous = accountConnected.current;
    if (previous !== value) accountEpoch.current++;
    accountConnected.current = value;
    setConnected(value);
    if (!value) {
      libraryRevision.current++;
      setBooks([]);
      setActive(undefined);
      setQuery("");
      setSeries("");
      setS({
        position: 0,
        duration: 0,
        speed: 1,
        paused: true,
        available: false,
      });
    } else if (!previous) {
      const revision = ++libraryRevision.current;
      void invoke<Book[]>("load_library")
        .then((cached) => {
          if (accountConnected.current && revision === libraryRevision.current)
            setBooks(cached);
        })
        .catch(() =>
          setStatus(
            "Saved library could not load. Use Sync library to retry; your local files have not been reset.",
          ),
        );
    }
  }, []);
  useEffect(() => {
    if (!desktop) return;
    let dead = false,
      stop: (() => void) | undefined;
    void listen("account-signed-out", () => accountChanged(false)).then((f) => {
      if (dead) f();
      else stop = f;
    });
    return () => {
      dead = true;
      stop?.();
    };
  }, [accountChanged]);
  useEffect(() => {
    if (!desktop) return;
    let cancelled = false;
    let stop: (() => void) | undefined;
    listen<string>("official-status", (e) => {
      if (
        !/^(Loading:|Page loaded:|Title launch remembered locally)/.test(
          e.payload,
        )
      )
        setStatus(e.payload);
    }).then((f) => {
      if (cancelled) f();
      else stop = f;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);
  useEffect(() => {
    if (!desktop) return;
    let dead = false;
    let stop: (() => void) | undefined;
    listen<Book>("player-book-details", (e) => {
      if (!accountConnected.current) return;
      setActive((b) => (b?.asin === e.payload.asin ? e.payload : b));
      setBooks((rows) =>
        rows.map((b) => (b.asin === e.payload.asin ? e.payload : b)),
      );
    }).then((f) => {
      if (dead) f();
      else stop = f;
    });
    return () => {
      dead = true;
      stop?.();
    };
  }, []);
  const command = (a: Action, v?: number) => run(() => adapter.command(a, v));
  const open = (b: Book) =>
    run(async () => {
      const epoch = accountEpoch.current;
      if (desktop && !accountConnected.current)
        throw Error("Sign in before starting playback.");
      if (desktop && !b.asin.startsWith("DEMO")) {
        await new OfficialAdapter().open(b);
        if (!accountConnected.current || epoch !== accountEpoch.current) return;
        setLive(true);
        setS({
          position: 0,
          duration: 0,
          speed: 1,
          paused: true,
          available: false,
        });
        setStatus("Starting playback; the mini player opens when audio starts");
      } else {
        if (live) throw Error("Demo titles cannot play in the official player");
        await adapter.open(b);
        setStatus("Demo playback ready");
      }
      setActive(b);
    });
  const setCurrent = (b: Book) =>
    run(async () => {
      await current.select(b.asin);
      setStatus(`${b.title} is now your current book`);
    });
  return (
    <main>
      <header>
        <div>
          <small>YOUR AUDIOBOOK LIBRARY</small>
          <h1>Listener’s Library</h1>
          {macExperimental && (
            <p role="status">
              Experimental macOS alpha · login and playback untested
            </p>
          )}
        </div>
        <div className="header-actions">
          <ThemePicker settings={settings} />
          <button
            disabled={!desktop || !connected || !active || !s.available}
            onClick={() => run(() => invoke("open_mini_player"))}
          >
            Open mini player
          </button>
        </div>
      </header>
      <MetadataPanel
        onConnection={accountChanged}
        onLibrary={(b) => {
          if (!accountConnected.current) return;
          libraryRevision.current++;
          setBooks(b);
          setSeries("");
          setQuery("");
        }}
        onStatus={setStatus}
      />
      <section className="toolbar">
        <span role="status">{status}</span>
        {desktop && connected && active && !s.available && (
          <div>
            <p>
              Complete any Amazon verification or click Listen now beside this
              book in the Audible window. Playback opens the mini player
              automatically.
            </p>
            <button
              onClick={() =>
                run(() => invoke("official_visibility", { visible: true }))
              }
            >
              Open Audible window
            </button>
            <button onClick={() => open(active)}>Retry this book</button>
          </div>
        )}
      </section>
      {connected && current.ready && (
        <CurrentBook
          asin={current.asin}
          book={currentTitle}
          saved={
            current.asin ? cardProgress.positions[current.asin] : undefined
          }
          livePosition={
            active?.asin === current.asin && s.available
              ? s.position
              : undefined
          }
          onListen={open}
          onClear={() =>
            run(async () => {
              await current.select(null);
              setStatus("Current book cleared");
            })
          }
          busy={current.busy}
        />
      )}
      {current.error && <p role="status">{current.error}</p>}
      {desktop && connected && !series && !query && (
        <ContinueShelf
          books={books.filter((b) => b.asin !== current.asin)}
          activity={activity}
          onListen={open}
          onStatus={setStatus}
          onSetCurrent={setCurrent}
          pinBusy={!current.ready || current.busy}
        />
      )}
      <div className="layout library-layout">
        <ExploreNav
          series={[
            ...new Set(accessibleBooks.map((b) => b.series).filter(Boolean)),
          ]}
          selected={series}
          onSelect={setSeries}
        />
        <section>
          <h2>{series || "Your library"}</h2>
          <div className="library-filters">
            <input
              aria-label="Search your library"
              placeholder="Search title or author"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <label>
              Sort{" "}
              <select
                aria-label="Sort library"
                disabled={desktop && !settings.ready}
                value={desktop ? settings.preferences.librarySort : demoSort}
                onChange={(e) =>
                  desktop
                    ? void settings.update({
                        librarySort: e.target.value as Sort,
                      })
                    : setDemoSort(e.target.value as Sort)
                }
              >
                <option value="title">Title A–Z</option>
                <option value="author">Author A–Z</option>
                <option value="series">Series & book order</option>
                <option value="recent">Recently listened here</option>
                <option value="shortest">Shortest first</option>
                <option value="longest">Longest first</option>
                <option value="progress">Most progress</option>
              </select>
            </label>
          </div>
          {settings.error && <p role="status">{settings.error}</p>}
          {!books.length && (
            <p>
              {connected
                ? "Your library is empty or still loading. Use Sync library to retry."
                : "Sign in to load your library automatically."}
            </p>
          )}
          {books.length > 0 && !visibleBooks.length && (
            <p>No books match. Clear your search or choose All titles.</p>
          )}
          <div className="books">
            {visibleBooks.map((b) => (
              <article key={b.asin}>
                <Cover book={b} />
                <h3>{b.title}</h3>
                {b.asin === current.asin && (
                  <span className="current-badge">Current book</span>
                )}
                <p>{b.author}</p>
                {b.series && (
                  <p>
                    {b.series} {b.volume ? `· Book ${b.volume}` : ""}
                  </p>
                )}
                <CardProgress
                  book={b}
                  saved={cardProgress.positions[b.asin]}
                  livePosition={
                    active?.asin === b.asin && s.available
                      ? s.position
                      : undefined
                  }
                />
                <button className="primary-action" onClick={() => open(b)}>
                  {desktop ? "Listen in mini player" : "Listen"}
                </button>
                <button
                  disabled={
                    !current.ready || current.busy || b.asin === current.asin
                  }
                  onClick={() => setCurrent(b)}
                >
                  {b.asin === current.asin
                    ? "✓ Current book"
                    : "Set as current"}
                </button>
                {b.series && (
                  <button onClick={() => setSeries(b.series)}>
                    See series
                  </button>
                )}
              </article>
            ))}
          </div>
        </section>
      </div>
      {connected && (
        <NotesPanel
          books={books}
          active={active}
          snapshot={s}
          onSeek={(p) => command("seek", p)}
        />
      )}
      <details className="app-details">
        <summary>Playback help & options</summary>
        <ShortcutHelp />
        <OfficialControls available={s.available} onStatus={setStatus} />
        {desktop && active && !active.asin.startsWith("DEMO") && (
          <ResumePanel
            book={active}
            available={s.available}
            paused={s.paused}
            duration={s.duration}
            onSeek={(p) => command("seek", p)}
            onChapters={(chapters) =>
              setActive((b) => (b ? { ...b, chapters } : b))
            }
            onStatus={setStatus}
          />
        )}
        <div className="playback-options">
          <ListeningControls book={active} snapshot={s} />
          <button disabled={!s.available} onClick={() => command("recover")}>
            Recover jump
          </button>
          <button
            disabled={!desktop || cardProgress.busy}
            onClick={() => cardProgress.refresh()}
          >
            Refresh saved positions
          </button>
        </div>
        {cardProgress.status && <p role="status">{cardProgress.status}</p>}
      </details>
      <PrivacyPanel />
      <details className="app-details">
        <summary>About & privacy</summary>
        <p>
          Listener’s Library · Version 0.2.1 ·{" "}
          {macExperimental ? "Experimental macOS alpha — untested" : "Beta"}.
          Unofficial and unaffiliated with Audible or Amazon. Currently supports
          the US marketplace.
        </p>
        <p>
          Sign in only on official Amazon/Audible pages. Account tokens and the
          library key are protected by {protectionName}. Library data, notes and
          preferences use an encrypted local database; website sign-in stays in
          its separate browser profile on this device. Playback and library sync
          connect to Audible. No app analytics are collected.
        </p>
        <p>
          Use Sign out before sharing your computer account. Cached books and
          notes remain on this device. Third-party license notices are included
          with the installation.
        </p>
      </details>
      {connected && active && (
        <footer>
          <VolumeControl
            value={desktop ? settings.preferences.volume : (s.volume ?? 1)}
            onChange={(v) =>
              desktop
                ? s.available
                  ? adapter.command("volume", v)
                  : settings.update({ volume: v })
                : adapter.command("volume", v)
            }
          />
          <strong>{active.title}</strong>
          <span>
            {s.available
              ? `${format(s.position)} / ${format(s.duration)}`
              : "Starting playback… Audible may request sign-in or a playback click."}
          </span>
          <div>
            <button
              disabled={!s.available}
              onClick={() => command("seek", Math.max(0, s.position - 15))}
            >
              −15s
            </button>
            <button
              disabled={!s.available}
              onClick={() => command(s.paused ? "play" : "pause")}
            >
              {s.paused ? "Play" : "Pause"}
            </button>
            <button
              disabled={!s.available}
              onClick={() =>
                command("seek", Math.min(s.duration, s.position + 15))
              }
            >
              +15s
            </button>
            <select
              aria-label="Playback speed"
              disabled={!s.available}
              value={s.speed}
              onChange={(e) => command("speed", Number(e.target.value))}
            >
              {[0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3].map((x) => (
                <option key={x} value={x}>
                  {x}×
                </option>
              ))}
            </select>
            <ChapterSelect
              book={active}
              position={s.position}
              duration={s.duration}
              available={s.available}
              seekable={s.seekable}
              onChapter={(position, title) =>
                live
                  ? run(() => new OfficialAdapter().chapter(position, title))
                  : command("seek", position)
              }
              onSeek={(p) => command("seek", p)}
            />
            <span>
              {active.chapters.length
                ? `Chapter ${chapterProgress(active, s.position)}% · `
                : ""}
              Finish{" "}
              {s.duration
                ? new Date(Date.now() + remaining(s) * 1000).toLocaleTimeString(
                    [],
                    { hour: "2-digit", minute: "2-digit" },
                  )
                : "—"}
            </span>
          </div>
        </footer>
      )}
    </main>
  );
}
function OfficialControls({
  available,
  onStatus,
}: {
  available: boolean;
  onStatus: (message: string) => void;
}) {
  const [loginStatus, setLoginStatus] = useState("");
  useEffect(() => {
    if (!desktop) return;
    let dead = false,
      stop: (() => void) | undefined;
    listen<string>("playback-login-status", (e) =>
      setLoginStatus(e.payload),
    ).then((f) => {
      if (dead) f();
      else stop = f;
    });
    return () => {
      dead = true;
      stop?.();
    };
  }, []);
  const run = async (command: string, args?: Record<string, unknown>) => {
    try {
      await invoke(command, args);
      onStatus(
        command === "open_login"
          ? "Opening the official library using the remembered website session. Sign in once if this is the first use of the shared profile."
          : command === "end_session"
            ? "Remembered website sign-in removed. The API connection is kept; use Sign out to remove both."
            : "Official player window updated",
      );
    } catch (e) {
      onStatus(String(e));
    }
  };
  if (!desktop) return null;
  return (
    <section className="metadata-panel">
      <div>
        <strong>Official Audible playback</strong>
        <p>
          Sign-in is remembered on this device. Open Audible if playback needs
          attention or your account needs verification.
        </p>
      </div>
      <button onClick={() => run("open_login")}>Open Audible library</button>
      <button onClick={() => run("official_visibility", { visible: true })}>
        Show official player
      </button>
      <button
        disabled={!available}
        onClick={() => run("official_visibility", { visible: false })}
      >
        Hide official player
      </button>
      <button onClick={() => run("end_session")}>Forget website sign-in</button>
      {loginStatus && <p role="status">{loginStatus}</p>}
    </section>
  );
}
function format(n: number) {
  return `${Math.floor(n / 3600)}:${String(Math.floor(n / 60) % 60).padStart(2, "0")}:${String(Math.floor(n) % 60).padStart(2, "0")}`;
}
applyTheme(cachedTheme());
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <ThemeManager />
      {new URLSearchParams(location.search).get("mini") === "1" ? (
        <MiniPlayer />
      ) : (
        <App />
      )}
    </ErrorBoundary>
  </React.StrictMode>,
);
