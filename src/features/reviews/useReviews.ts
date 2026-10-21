/**
 * Account-scoped local review loading and optimistic saves. The backend
 * generation pins storage identity, UI revisions reject stale completions, and
 * the browser demo keeps only in-memory values.
 */

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { desktop } from "../../playback";
import { reviewError, type Review } from "./reviewModel";

type Entry = Review & { asin: string };

/**
 * Loads/saves only for the captured native account generation. expected is the
 * original saved value used for optimistic concurrency, not the edited draft.
 */
export function useReviews(connected: boolean) {
  const [entries, setEntries] = useState<Record<string, Review>>({});
  const [ready, setReady] = useState(false),
    [error, setError] = useState("");
  // Native generation pins storage ownership; revision guards this mounted UI.
  // Both are necessary: hiding stale results alone cannot authorize a DB write.
  const session = useRef<number | undefined>(undefined),
    revision = useRef(0);
  useEffect(() => {
    const id = ++revision.current;
    session.current = undefined;
    setEntries({});
    setReady(false);
    setError("");
    if (!connected) return;
    if (!desktop) {
      setReady(true);
      return;
    }
    void (async () => {
      try {
        const generation = await invoke<number>("notes_session");
        const rows = await invoke<Entry[]>("list_reviews", { generation });
        if (id !== revision.current) return;
        session.current = generation;
        setEntries(
          Object.fromEntries(
            rows.map(({ asin, rating, text }) => [asin, { rating, text }]),
          ),
        );
        setReady(true);
      } catch (e) {
        if (id === revision.current) setError(String(e));
      }
    })();
    return () => {
      revision.current++;
      session.current = undefined;
    };
  }, [connected]);
  async function save(
    asin: string,
    review: Review | null,
    expected: Review | null,
  ) {
    if (!ready || !connected || (desktop && session.current === undefined))
      throw Error("Local reviews are not ready. Reopen the app to retry.");
    if (review) {
      const error = reviewError(review);
      if (error) throw Error(error);
    }
    const id = revision.current;
    if (desktop)
      await invoke("save_review", {
        generation: session.current,
        asin,
        review,
        expected,
      });
    if (id !== revision.current)
      throw Error("The account changed. Your review is no longer open.");
    setEntries((rows) => {
      const next = { ...rows };
      if (review) next[asin] = { ...review, text: review.text.trim() };
      else delete next[asin];
      return next;
    });
  }
  return { entries, ready, error, save };
}
