/**
 * Explicit current-book selection and its library card. Selection is distinct
 * from the title currently playing; account-change events clear state before
 * reloading the appropriate store.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "./playback";
import type { Book } from "./model";
import type { CardPositions } from "./useCardPositions";
import { Cover } from "./Cover";
import { CardProgress } from "./CardProgress";
/**
 * Loads the explicit account pin and clears it before account-event reloads. In
 * the browser demo only, localStorage simulates this preference.
 */
export function useCurrentBook() {
  const [asin, setAsin] = useState<string | null>(null),
    [ready, setReady] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const alive = useRef(false),
    version = useRef(0);
  const load = useCallback(async () => {
    const id = ++version.current;
    const value = desktop
      ? await invoke<string | null>("get_current_book")
      : localStorage.getItem("demo-current-book");
    if (alive.current && id === version.current) {
      setAsin(value);
      setReady(true);
      setError("");
    }
  }, []);
  useEffect(() => {
    alive.current = true;
    let dead = false;
    const stops: (() => void)[] = [];
    const reload = () =>
      load().catch((e) => {
        if (!dead) setError(String(e));
      });
    if (desktop) {
      for (const event of [
        "current-book-changed",
        "api-account-changed",
        "account-signed-out",
      ])
        void listen(event, () => {
          version.current++;
          setAsin(null);
          void reload();
        })
          .then((f) => {
            if (dead) f();
            else stops.push(f);
          })
          .catch(() => {});
      void reload();
    } else void reload();
    return () => {
      dead = true;
      alive.current = false;
      version.current++;
      stops.forEach((f) => f());
    };
  }, [load]);
  const select = async (value: string | null) => {
    setBusy(true);
    try {
      if (desktop) await invoke("set_current_book", { asin: value });
      else if (value) localStorage.setItem("demo-current-book", value);
      else localStorage.removeItem("demo-current-book");
      await load();
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  return { asin, ready, busy, error, select };
}
/**
 * Shows the pinned book independently of the active player and Continue shelf,
 * with explicit resume and clear actions.
 */
export function CurrentBook({
  asin,
  book,
  saved,
  livePosition,
  onListen,
  onClear,
  busy,
}: {
  asin: string | null;
  book?: Book;
  saved?: CardPositions[string];
  livePosition?: number;
  onListen: (book: Book) => void;
  onClear: () => void;
  busy: boolean;
}) {
  return (
    <section className="current-book">
      <div className="section-heading">
        <h2>Current book</h2>
        <span className="current-label">Chosen by you</span>
      </div>
      {book ? (
        <article>
          <Cover book={book} />
          <div className="current-book-details">
            <h3>{book.title}</h3>
            <p>{book.author}</p>
            <CardProgress
              book={book}
              saved={saved}
              livePosition={livePosition}
            />
            <button className="primary-action" onClick={() => onListen(book)}>
              Resume in mini player
            </button>
            <button disabled={busy} onClick={onClear}>
              Clear current book
            </button>
          </div>
        </article>
      ) : asin ? (
        <p>
          Your current book isn’t in the cached library. Sync your library or{" "}
          <button disabled={busy} onClick={onClear}>
            clear the selection
          </button>
          .
        </p>
      ) : (
        <p className="empty-shelf">
          Choose <strong>Set as current</strong> on a title to keep your main
          read here. It stays until you change or clear it.
        </p>
      )}
      <p className="current-book-hint">
        Playing another title won’t change your selection.
      </p>
    </section>
  );
}
