/**
 * Account/book-scoped listening history and recovery UI. Recovery is an
 * explicit choice between local and Audible positions, not a claim that either
 * service exposes the newest listening timestamp.
 */

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Book } from "./model";
import { desktop } from "./playback";
import { clock } from "./listening";

export type HistoryEntry = {
  id: number;
  position: number;
  duration: number;
  at: number;
  kind: string;
};
type HistoryStatus = {
  asin: string;
  generation: number;
  waiting: boolean;
  conflict: { pc: HistoryEntry; audible: number } | null;
  entries: HistoryEntry[];
};
/**
 * Loads history for one book and refreshes on native history events. Revision
 * checks and cleanup prevent a previous book's recovery state from leaking into
 * the next selection.
 */
export function useListeningHistory(book?: Book) {
  const [status, setStatus] = useState<HistoryStatus>(),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const version = useRef(0),
    readVersion = useRef(0),
    refresh = useRef(async () => {});
  useEffect(() => {
    const id = ++version.current;
    setStatus(undefined);
    setError("");
    setBusy(false);
    if (!desktop || !book) return;
    const reload = async () => {
      const read = ++readVersion.current;
      try {
        const value = await invoke<HistoryStatus>("listening_history", {
          asin: book.asin,
        });
        if (
          id === version.current &&
          read === readVersion.current &&
          value.asin === book.asin
        ) {
          setStatus(value);
          setError("");
        }
      } catch (e) {
        if (id === version.current && read === readVersion.current)
          setError(String(e));
      }
    };
    refresh.current = reload;
    let stop: (() => void) | undefined;
    void listen("listening-history-changed", () => void reload())
      .then((f) => {
        if (id !== version.current) f();
        else stop = f;
      })
      .then(() => {
        if (id === version.current) void reload();
      })
      .catch((e) => {
        if (id === version.current) setError(String(e));
      });
    return () => {
      version.current++;
      stop?.();
    };
  }, [book?.asin]);
  const choose = async (id?: number) => {
    if (!status || busy) return;
    const request = version.current;
    setBusy(true);
    setError("");
    try {
      await invoke(
        id === undefined
          ? "accept_audible_position"
          : "restore_listening_position",
        {
          asin: status.asin,
          generation: status.generation,
          ...(id === undefined ? {} : { id }),
        },
      );
      if (request === version.current) await refresh.current();
    } catch (e) {
      if (request === version.current) setError(String(e));
    } finally {
      if (request === version.current) setBusy(false);
    }
  };
  return { status, error, busy, choose };
}
function date(at: number) {
  return new Date(at).toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
function label(kind: string) {
  return (
    (
      {
        listening: "Listening",
        pause: "Last heard before pause",
        close: "Last heard before closing",
        switch: "Last heard before switching",
        "chosen-position": "Position chosen",
        "before-position-change": "Last heard before changing position",
        "previous-PC-checkpoint": "Checkpoint from older build",
        "previous-listening": "Listening from older build",
      } as Record<string, string>
    )[kind] ?? "Saved position"
  );
}
/**
 * Requires an explicit recovery choice when positions conflict and exposes
 * dated PC checkpoints; restores are confirmed natively before success is
 * shown.
 */
export function ListeningHistoryPanel({
  history,
  available,
}: {
  history: ReturnType<typeof useListeningHistory>;
  available: boolean;
}) {
  const { status, error, busy, choose } = history;
  return (
    <div className="listening-history">
      {status?.conflict && (
        <div className="resume-conflict" role="status">
          <strong>Saved positions differ · playback paused</strong>
          <p>
            Choose where to continue. Audible’s position has no verified
            listening time, so we cannot safely call it newer.
          </p>
          <button disabled={busy || !available} onClick={() => void choose(0)}>
            Resume PC · {clock(status.conflict.pc.position)}
            <span>
              {label(status.conflict.pc.kind)} · {date(status.conflict.pc.at)}
            </span>
          </button>
          <button disabled={busy || !available} onClick={() => void choose()}>
            Use Audible · {clock(status.conflict.audible)}
            <span>
              Position opened by the official player · listening time unknown
            </span>
          </button>
        </div>
      )}
      {status?.waiting && !status.conflict && (
        <p role="status">Checking saved progress before playback…</p>
      )}
      {error && <p role="alert">{error}</p>}
      {busy && <p role="status">Applying saved position…</p>}
      <details className="history-entries" open={!status?.conflict}>
        <summary>Recent positions on this PC</summary>
        <p>
          Encrypted local recovery log. Up to 100 entries per book and 3,000 per
          account; entries older than 90 days are pruned on the next save.
          Earlier checkpoints remain available until replaced by listening or a
          deliberate choice.
        </p>
        {status?.entries.length ? (
          <ol>
            {status.entries.map((e) => (
              <li key={e.id}>
                <button
                  disabled={busy || !available}
                  onClick={() => void choose(e.id)}
                >
                  <strong>{clock(e.position)}</strong>
                  <span>
                    {date(e.at)} · {label(e.kind)}
                  </span>
                </button>
              </li>
            ))}
          </ol>
        ) : (
          <p>
            No recovery entries yet. Listening here will save them
            automatically.
          </p>
        )}
      </details>
    </div>
  );
}
