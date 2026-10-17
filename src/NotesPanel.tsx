import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Book, Snapshot } from "./model";
import { desktop } from "./playback";
import { changeDraft, clearDraft, type Drafts } from "./noteDrafts";
type Note = { id: number; asin: string; position: number; text: string };
const clock = (p: number) =>
  `${Math.floor(p / 3600)}:${String(Math.floor(p / 60) % 60).padStart(2, "0")}:${String(Math.floor(p) % 60).padStart(2, "0")}`;

export function NotesPanel({
  books,
  active,
  snapshot,
  onSeek,
}: {
  books: Book[];
  active?: Book;
  snapshot: Snapshot;
  onSeek: (position: number) => void;
}) {
  const [notes, setNotes] = useState<Note[]>([]),
    [selected, setSelected] = useState(active?.asin ?? ""),
    [drafts, setDrafts] = useState<Drafts>({}),
    [busy, setBusy] = useState(false),
    [status, setStatus] = useState(""),
    [editing, setEditing] = useState<number>(),
    [editText, setEditText] = useState(""),
    [deleting, setDeleting] = useState<number>(),
    [confirmation, setConfirmation] = useState("");
  const context = useRef<number | null>(null);
  const alive = useRef(true),
    revision = useRef(0);
  const load = async () => {
    const id = ++revision.current;
    if (context.current === null) return;
    const value = await invoke<Note[]>("list_notes", {
      generation: context.current,
    });
    if (alive.current && id === revision.current) setNotes(value);
  };
  useEffect(() => {
    if (!desktop) return;
    alive.current = true;
    let dead = false;
    let stop: (() => void) | undefined;
    void invoke<number>("notes_session")
      .then((generation) => {
        if (!dead && alive.current) {
          context.current = generation;
          return load();
        }
      })
      .catch(() => {
        if (!dead && alive.current)
          setStatus(
            "Notes could not load. Close and reopen the app; your saved notes have not been reset.",
          );
      });
    void listen(
      "notes-changed",
      () =>
        void load().catch(() => {
          if (!dead && alive.current)
            setStatus("Could not refresh notes. Reopen this window.");
        }),
    )
      .then((f) => {
        if (!dead && alive.current) stop = f;
        else f();
      })
      .catch(() => {
        if (!dead && alive.current)
          setStatus(
            "Automatic note refresh could not connect. Reopen the app before editing notes.",
          );
      });
    return () => {
      dead = true;
      alive.current = false;
      revision.current++;
      stop?.();
    };
  }, []);
  useEffect(() => {
    if (active) setSelected(active.asin);
  }, [active?.asin]);
  const run = async (task: () => Promise<void>) => {
    setBusy(true);
    try {
      if (context.current === null)
        throw Error(
          "Notes are still connecting. Reopen the app if this persists.",
        );
      await task();
    } catch (e) {
      if (alive.current) setStatus(String(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  const draft = drafts[selected],
    canCapture = !!active && active.asin === selected && snapshot.available;
  const save = () =>
    run(async () => {
      if (!draft?.text.trim() || !canCapture) return;
      const asin = selected,
        text = draft.text,
        position = draft.position;
      await invoke("save_note", {
        asin,
        text,
        position,
        generation: context.current,
      });
      if (!alive.current) return;
      setDrafts((d) => clearDraft(d, asin, text));
      setStatus("Note saved on this PC.");
      await load();
    });
  const options = [
    ...new Set([...books.map((b) => b.asin), ...notes.map((n) => n.asin)]),
  ];
  if (!desktop) return null;
  return (
    <details className="app-details">
      <summary>Notes & backups</summary>
      <label>
        Book{" "}
        <select
          aria-label="Book for notes"
          value={selected}
          onChange={(e) => {
            setSelected(e.target.value);
            setEditing(undefined);
            setDeleting(undefined);
          }}
        >
          <option value="" disabled>
            Choose a book
          </option>
          {options.map((asin) => (
            <option key={asin} value={asin}>
              {books.find((b) => b.asin === asin)?.title ?? asin}
            </option>
          ))}
        </select>
      </label>
      <p>
        Read your notes without starting playback. New notes keep the timestamp
        from when you begin typing.
      </p>
      <textarea
        aria-label="New timestamped note"
        maxLength={10000}
        disabled={!canCapture || busy}
        value={draft?.text ?? ""}
        onChange={(e) =>
          setDrafts((d) =>
            changeDraft(d, selected, e.target.value, snapshot.position),
          )
        }
        placeholder={
          canCapture
            ? "Write a thought about this book"
            : "Start this book to add a timestamped note"
        }
      />
      {draft?.text && (
        <p>
          Draft for this book · {clock(draft.position)}. Unsaved drafts are
          cleared when you close the app or sign out.
        </p>
      )}
      <button
        disabled={busy || !canCapture || !draft?.text.trim()}
        onClick={save}
      >
        Save note
      </button>
      {notes
        .filter((n) => n.asin === selected)
        .map((n) => (
          <article className="note" key={n.id}>
            <button
              disabled={!canCapture}
              onClick={() => onSeek(n.position)}
              aria-label={`Jump to note at ${clock(n.position)}`}
            >
              {clock(n.position)}
            </button>
            {editing === n.id ? (
              <>
                <textarea
                  aria-label="Edit note"
                  maxLength={10000}
                  value={editText}
                  onChange={(e) => setEditText(e.target.value)}
                />
                <button
                  disabled={busy || !editText.trim()}
                  onClick={() =>
                    void run(async () => {
                      await invoke("edit_note", {
                        generation: context.current,
                        id: n.id,
                        asin: n.asin,
                        text: editText,
                        expected: n.text,
                      });
                      if (!alive.current) return;
                      setEditing(undefined);
                      await load();
                    })
                  }
                >
                  Save edit
                </button>
                <button onClick={() => setEditing(undefined)}>
                  Cancel edit
                </button>
              </>
            ) : (
              <>
                <p className="note-text">{n.text}</p>
                <button
                  onClick={() => {
                    setEditing(n.id);
                    setEditText(n.text);
                  }}
                >
                  Edit
                </button>
                <button onClick={() => setDeleting(n.id)}>Delete</button>
              </>
            )}
            {deleting === n.id && (
              <p>
                Delete this note permanently?{" "}
                <button
                  disabled={busy}
                  onClick={() =>
                    void run(async () => {
                      await invoke("delete_note", {
                        generation: context.current,
                        expected: n.text,
                        id: n.id,
                        asin: n.asin,
                      });
                      if (!alive.current) return;
                      setDeleting(undefined);
                      await load();
                    })
                  }
                >
                  Delete note
                </button>
                <button onClick={() => setDeleting(undefined)}>
                  Keep note
                </button>
              </p>
            )}
          </article>
        ))}
      <details>
        <summary>Export notes or recover notes from older versions</summary>
        <p>
          Exports contain readable text and book identifiers. They do not
          contain your login or encryption key. Choose a private folder; a
          cloud-synced folder may upload the file.
        </p>
        <p>
          Old notes were not assigned to an account. Recovery exports the old
          store without merging it into this account. It may include notes from
          other accounts used in earlier versions. Only export old notes if you
          own them and control this Windows account.
        </p>
        <label>
          Type EXPORT NOTES for this account, or EXPORT OLD NOTES for the old
          store{" "}
          <input
            aria-label="Confirm notes export"
            value={confirmation}
            onChange={(e) => setConfirmation(e.target.value)}
          />
        </label>
        <button
          disabled={
            busy || !["EXPORT NOTES", "EXPORT OLD NOTES"].includes(confirmation)
          }
          onClick={() =>
            void run(async () => {
              const saved = await invoke<boolean>("export_notes", {
                generation: context.current,
                old: confirmation === "EXPORT OLD NOTES",
                confirmation,
              });
              if (alive.current) {
                setStatus(
                  saved
                    ? "Notes exported to the file you selected. Keep it private."
                    : "Export canceled.",
                );
                setConfirmation("");
              }
            })
          }
        >
          Choose export file…
        </button>
      </details>
      {status && <p role="status">{status}</p>}
    </details>
  );
}
