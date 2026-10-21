import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Book, Snapshot } from "./model";
import { desktop } from "./playback";
import { chapterInfo } from "./playerInfo";
import "./highlights.css";
import {
  excerpt,
  parseStamp,
  passageError,
  passageOnly,
  stamp,
  type Highlight,
  type Passage,
  type PassageKind,
} from "./passages";

type Draft = {
  kind: PassageKind;
  start: string;
  end: string;
  text: string;
  editing?: Highlight;
};
const labels = { bookmark: "Bookmark", clip: "Clip", note: "Note" };
export function HighlightsPanel({
  books,
  active,
  snapshot,
}: {
  books: Book[];
  active?: Book;
  snapshot: Snapshot;
}) {
  const [items, setItems] = useState<Highlight[]>([]),
    [selected, setSelected] = useState(active?.asin ?? "*"),
    [filter, setFilter] = useState<PassageKind | "all">("all"),
    [search, setSearch] = useState(""),
    [sort, setSort] = useState("position"),
    [drafts, setDrafts] = useState<Record<string, Draft>>({}),
    [busy, setBusy] = useState(false),
    [status, setStatus] = useState(""),
    [deleting, setDeleting] = useState<number>(),
    [confirmation, setConfirmation] = useState(""),
    [returnPoint, setReturnPoint] = useState<{
      asin: string;
      position: number;
    }>();
  const generation = useRef<number | null>(null),
    revision = useRef(0),
    alive = useRef(false),
    section = useRef<HTMLDetailsElement>(null);
  async function load() {
    const version = ++revision.current,
      account = generation.current;
    if (account === null) return;
    const value = await invoke<Highlight[]>("list_notes", {
      generation: account,
    });
    if (
      alive.current &&
      version === revision.current &&
      account === generation.current
    )
      setItems(value);
  }
  useEffect(() => {
    if (!desktop) return;
    alive.current = true;
    let dead = false;
    const stops: (() => void)[] = [];
    const keep = (f: () => void) => (dead ? f() : stops.push(f));
    void invoke<number>("notes_session")
      .then((g) => {
        if (!dead) {
          generation.current = g;
          return load();
        }
      })
      .catch(() => {
        if (!dead)
          setStatus(
            "Saved items could not load. Your data has not been reset.",
          );
      });
    void listen("notes-changed", () => {
      void load().catch(() => {
        if (!dead)
          setStatus(
            "Could not refresh saved items. Try reopening this window.",
          );
      });
    })
      .then(keep)
      .catch(() => {
        if (!dead)
          setStatus("Live refresh is unavailable. Reopen before editing.");
      });
    void listen("show-highlights", () => {
      if (section.current) {
        section.current.open = true;
        section.current.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    })
      .then(keep)
      .catch(() => {});
    return () => {
      dead = true;
      alive.current = false;
      generation.current = null;
      revision.current++;
      stops.forEach((f) => f());
    };
  }, []);
  useEffect(() => {
    if (active) setSelected(active.asin);
    setReturnPoint(undefined);
    setDeleting(undefined);
  }, [active?.asin]);
  if (!desktop) return null;
  const book = books.find((b) => b.asin === selected),
    draft = drafts[selected];
  const canCapture = !!active && active.asin === selected && snapshot.available;
  const scoped = items.filter((n) => selected === "*" || n.asin === selected);
  const visible = scoped
    .filter(
      (n) =>
        (filter === "all" || n.kind === filter) &&
        `${n.text} ${books.find((b) => b.asin === n.asin)?.title ?? ""}`
          .toLocaleLowerCase()
          .includes(search.toLocaleLowerCase()),
    )
    .sort((a, b) =>
      sort === "recent"
        ? (b.created_ms ?? 0) - (a.created_ms ?? 0) || b.id - a.id
        : a.asin.localeCompare(b.asin) ||
          a.position - b.position ||
          a.id - b.id,
    );
  async function run(task: () => Promise<void>) {
    if (generation.current === null) {
      setStatus("Saved items are still connecting.");
      return;
    }
    setBusy(true);
    setStatus("");
    try {
      await task();
    } catch (e) {
      if (alive.current) setStatus(String(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  function begin(kind: PassageKind) {
    if (!canCapture) return;
    const [start, end] = excerpt(snapshot.position, snapshot.duration);
    setDrafts((d) => ({
      ...d,
      [selected]: {
        kind,
        start: stamp(kind === "clip" ? start : snapshot.position),
        end: stamp(end),
        text: "",
      },
    }));
  }
  function patch(values: Partial<Draft>) {
    setDrafts((d) => ({ ...d, [selected]: { ...d[selected], ...values } }));
  }
  function discard(asin = selected) {
    setDrafts((d) => {
      const next = { ...d };
      delete next[asin];
      return next;
    });
  }
  function save() {
    if (!draft) return;
    const start = parseStamp(draft.start),
      end = parseStamp(draft.end);
    const passage: Passage = {
      kind: draft.kind,
      position: start ?? NaN,
      end_position: draft.kind === "clip" ? (end ?? NaN) : null,
      text: draft.text,
    };
    const error = passageError(passage, book?.duration);
    if (error) {
      setStatus(error);
      return;
    }
    const asin = selected,
      savedDraft = draft;
    void run(async () => {
      await invoke("save_highlight", {
        generation: generation.current,
        asin,
        passage,
        id: draft.editing?.id ?? null,
        expected: draft.editing ? passageOnly(draft.editing) : null,
      });
      if (!alive.current) return;
      setDrafts((d) => {
        if (d[asin] !== savedDraft) return d;
        const next = { ...d };
        delete next[asin];
        return next;
      });
      setStatus("Saved securely on this PC.");
      await load();
    });
  }
  return (
    <details className="app-details highlights" open ref={section}>
      <summary>
        Bookmarks, clips & notes{" "}
        <span className="highlight-total">{items.length}</span>
      </summary>
      <header className="highlights-heading">
        <div>
          <h2>Keep the good parts.</h2>
          <p>Save a moment, replay a passage, or leave yourself a thought.</p>
        </div>
        <span className="highlight-local">LOCAL · ENCRYPTED</span>
      </header>
      <div className="highlight-toolbar">
        <label>
          Book
          <select
            aria-label="Book for saved items"
            disabled={busy}
            value={selected}
            onChange={(e) => {
              setSelected(e.target.value);
              setDeleting(undefined);
            }}
          >
            <option value="*">All books</option>
            {[
              ...new Set([
                ...books.map((b) => b.asin),
                ...items.map((n) => n.asin),
              ]),
            ].map((asin) => (
              <option key={asin} value={asin}>
                {books.find((b) => b.asin === asin)?.title ?? asin}
              </option>
            ))}
          </select>
        </label>
        <label>
          Search
          <input
            type="search"
            placeholder="Find a thought or passage…"
            aria-label="Search saved items"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <label>
          Order
          <select
            aria-label="Sort saved items"
            value={sort}
            onChange={(e) => setSort(e.target.value)}
          >
            <option value="position">Book position</option>
            <option value="recent">Newest saved</option>
          </select>
        </label>
      </div>
      <div
        className="highlight-filters"
        role="group"
        aria-label="Filter saved items"
      >
        {(["all", "bookmark", "clip", "note"] as const).map((kind) => (
          <button
            key={kind}
            aria-pressed={filter === kind}
            onClick={() => setFilter(kind)}
          >
            {kind === "all" ? "Everything" : `${labels[kind]}s`}{" "}
            <span>
              {scoped.filter((n) => kind === "all" || n.kind === kind).length}
            </span>
          </button>
        ))}
      </div>
      {!draft ? (
        <div className="highlight-create">
          <button
            disabled={!canCapture || busy}
            onClick={() => begin("bookmark")}
          >
            ◇ Add bookmark
          </button>
          <button disabled={!canCapture || busy} onClick={() => begin("clip")}>
            ✂ Clip last 30 seconds
          </button>
          <button disabled={!canCapture || busy} onClick={() => begin("note")}>
            ＋ Write note
          </button>
          {!canCapture && (
            <small>
              Open a book to capture its current position. Saved items remain
              readable without playback.
            </small>
          )}
        </div>
      ) : (
        <div className="highlight-composer">
          <header>
            <strong>
              {draft.editing ? "Edit" : "New"}{" "}
              {labels[draft.kind].toLowerCase()}
            </strong>
            <button disabled={busy} onClick={() => discard()}>
              Cancel
            </button>
          </header>
          <p className="highlight-book-title">{book?.title ?? selected}</p>
          <div className="highlight-times">
            <label>
              {draft.kind === "clip" ? "Start" : "Position"}
              <input
                aria-label="Saved item start time"
                value={draft.start}
                disabled={busy}
                onChange={(e) => patch({ start: e.target.value })}
                placeholder="0:00:00"
              />
            </label>
            {draft.kind === "clip" && (
              <label>
                End
                <input
                  aria-label="Clip end time"
                  value={draft.end}
                  disabled={busy}
                  onChange={(e) => patch({ end: e.target.value })}
                  placeholder="0:00:30"
                />
              </label>
            )}
            <button
              disabled={!canCapture || busy}
              onClick={() => patch({ start: stamp(snapshot.position) })}
            >
              Start here
            </button>
            {draft.kind === "clip" && (
              <button
                disabled={!canCapture || busy}
                onClick={() => patch({ end: stamp(snapshot.position) })}
              >
                End here
              </button>
            )}
          </div>
          <label>
            {draft.kind === "note" ? "Your note" : "Label or note (optional)"}
            <textarea
              autoFocus
              aria-label="Saved item note"
              value={draft.text}
              maxLength={10000}
              disabled={busy}
              onChange={(e) => patch({ text: e.target.value })}
              placeholder="What makes this moment worth keeping?"
            />
          </label>
          <small>
            {draft.kind === "clip"
              ? "Clips save a start and end time, up to 5 minutes. No audio file is copied."
              : "The timestamp stays fixed while you write."}{" "}
            Unsaved drafts stay with their book until you close the app or sign
            out.
          </small>
          <button className="highlight-save" disabled={busy} onClick={save}>
            Save {labels[draft.kind].toLowerCase()}
          </button>
        </div>
      )}
      {returnPoint && active?.asin === returnPoint.asin && (
        <div className="highlight-return">
          <span>Before replay: {stamp(returnPoint.position)}</span>
          <button
            disabled={!snapshot.available || busy}
            onClick={() =>
              void run(async () => {
                await invoke("seek_highlight_position", {
                  generation: generation.current,
                  asin: returnPoint.asin,
                  position: returnPoint.position,
                });
                if (alive.current) {
                  setReturnPoint(undefined);
                  setStatus("Returned to your earlier listening position.");
                }
              })
            }
          >
            Return to listening
          </button>
        </div>
      )}
      <div className="highlight-list">
        {visible.length === 0 && (
          <div className="highlight-empty">
            <span>◇</span>
            <h3>
              {items.length
                ? "No matching moments"
                : "Your favorite moments belong here"}
            </h3>
            <p>
              {items.length
                ? "Try another filter, book, or search."
                : "Bookmark a scene, clip a favorite passage, or save a thought as you listen."}
            </p>
          </div>
        )}
        {visible.map((n) => {
          const savedBook = books.find((b) => b.asin === n.asin),
            chapter = savedBook
              ? chapterInfo(savedBook, { ...snapshot, position: n.position })
              : undefined,
            playable = active?.asin === n.asin && snapshot.available;
          return (
            <article
              className={`highlight-card highlight-${n.kind}`}
              key={n.id}
            >
              <div className="highlight-card-icon" aria-hidden="true">
                {n.kind === "clip" ? "✂" : n.kind === "note" ? "✎" : "◇"}
              </div>
              <div className="highlight-card-content">
                <div className="highlight-meta">
                  <span className="highlight-kind">{labels[n.kind]}</span>
                  <span>
                    {stamp(n.position)}
                    {n.end_position !== null
                      ? ` → ${stamp(n.end_position)}`
                      : ""}
                  </span>
                  {n.end_position !== null && (
                    <span>{Math.round(n.end_position - n.position)}s</span>
                  )}
                </div>
                <h3>
                  {n.text.trim() && n.text !== "Bookmark"
                    ? n.text.split("\n")[0]
                    : (chapter?.title ?? labels[n.kind])}
                </h3>
                {n.text.includes("\n") && (
                  <p className="note-text">
                    {n.text.slice(n.text.indexOf("\n") + 1)}
                  </p>
                )}
                <p className="highlight-context">
                  {savedBook?.title ?? n.asin}
                  {chapter ? ` · ${chapter.title}` : ""}
                </p>
                {n.created_ms && (
                  <time dateTime={new Date(n.created_ms).toISOString()}>
                    {new Date(n.created_ms).toLocaleDateString(undefined, {
                      month: "short",
                      day: "numeric",
                      year: "numeric",
                    })}
                  </time>
                )}
                <div className="highlight-card-actions">
                  {n.kind === "clip" && (
                    <button
                      disabled={!playable || busy}
                      onClick={() =>
                        void run(async () => {
                          const point = {
                            asin: n.asin,
                            position: snapshot.position,
                          };
                          await invoke("play_saved_clip", {
                            generation: generation.current,
                            asin: n.asin,
                            id: n.id,
                          });
                          if (!alive.current) return;
                          setReturnPoint((p) =>
                            p?.asin === point.asin ? p : point,
                          );
                          setStatus(
                            "Clip requested. Playback pauses at the end; Return to listening restores your earlier position.",
                          );
                        })
                      }
                    >
                      ▶ Play clip
                    </button>
                  )}
                  <button
                    disabled={!playable || busy}
                    onClick={() =>
                      void run(async () => {
                        await invoke("seek_highlight_position", {
                          generation: generation.current,
                          asin: n.asin,
                          position: n.position,
                        });
                        if (alive.current)
                          setStatus("Moved to the saved position.");
                      })
                    }
                  >
                    Go to bookmark
                  </button>
                  <button
                    disabled={busy || !!draft || !!drafts[n.asin]}
                    title={
                      drafts[n.asin]
                        ? "Finish this book’s draft before editing"
                        : "Edit saved item"
                    }
                    onClick={() => {
                      setSelected(n.asin);
                      setDrafts((d) => ({
                        ...d,
                        [n.asin]: {
                          kind: n.kind,
                          start: stamp(n.position),
                          end: stamp(n.end_position ?? n.position),
                          text:
                            n.text === "Bookmark" && n.kind === "bookmark"
                              ? ""
                              : n.text,
                          editing: n,
                        },
                      }));
                    }}
                  >
                    Edit
                  </button>
                  <button disabled={busy} onClick={() => setDeleting(n.id)}>
                    Delete
                  </button>
                </div>
                {!playable && <small>Open this book to replay or jump.</small>}
                {deleting === n.id && (
                  <div
                    className="highlight-delete"
                    role="group"
                    aria-label="Confirm deletion"
                  >
                    <span>
                      Permanently delete this {labels[n.kind].toLowerCase()}?
                    </span>
                    <button
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          await invoke("delete_highlight", {
                            generation: generation.current,
                            asin: n.asin,
                            id: n.id,
                            expected: passageOnly(n),
                          });
                          if (!alive.current) return;
                          setDeleting(undefined);
                          await load();
                        })
                      }
                    >
                      Delete permanently
                    </button>
                    <button
                      disabled={busy}
                      onClick={() => setDeleting(undefined)}
                    >
                      Keep it
                    </button>
                  </div>
                )}
              </div>
            </article>
          );
        })}
      </div>
      <div className="highlight-footer">
        <p>
          Saved only in this app on this Windows account. Clips replay through
          the official player, move the timeline and pause at the end. They do
          not sync to Audible’s mobile clips.
        </p>
        <details>
          <summary>Export & older notes</summary>
          <p>
            Exports include readable notes, book identifiers and clip ranges.
            Keep them private. Old notes may belong to accounts used in earlier
            versions; export them only if you own them and control this Windows
            account.
          </p>
          <label>
            Type EXPORT NOTES or EXPORT OLD NOTES
            <input
              aria-label="Confirm notes export"
              value={confirmation}
              onChange={(e) => setConfirmation(e.target.value)}
            />
          </label>
          <button
            disabled={
              busy ||
              !["EXPORT NOTES", "EXPORT OLD NOTES"].includes(confirmation)
            }
            onClick={() =>
              void run(async () => {
                const saved = await invoke<boolean>("export_notes", {
                  generation: generation.current,
                  old: confirmation === "EXPORT OLD NOTES",
                  confirmation,
                });
                if (!alive.current) return;
                setStatus(
                  saved
                    ? "Saved a readable export. Keep it private."
                    : "Export cancelled.",
                );
                setConfirmation("");
              })
            }
          >
            Choose export file…
          </button>
        </details>
      </div>
      {status && (
        <p className="highlight-status" role="status">
          {status}
        </p>
      )}
    </details>
  );
}
