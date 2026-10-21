/**
 * Manual saved-position and chapter refresh controls for an active book.
 * Request generations keep responses for a previous book from replacing the
 * current book's metadata.
 */

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Book, CloudMetadata } from "./model";
type Saved = { position: number; updated: number };
/**
 * Reads local/cloud saved positions and refreshed chapter metadata without
 * automatically moving playback. A user action chooses a resume location.
 */
export function ResumePanel({
  book,
  available,
  paused,
  duration,
  onSeek,
  onChapters,
  onStatus,
}: {
  book: Book;
  available: boolean;
  paused: boolean;
  duration: number;
  onSeek: (position: number) => void;
  onChapters: (chapters: number[]) => void;
  onStatus: (message: string) => void;
}) {
  const [local, setLocal] = useState<Saved | null>(null),
    [cloud, setCloud] = useState<number | null>(null),
    [busy, setBusy] = useState(false),
    [detailStatus, setDetailStatus] = useState(
      "Loading chapters and saved progress…",
    );
  const generation = useRef(0),
    readRef = useRef<(silent?: boolean) => Promise<void>>(async () => {}),
    pausedRef = useRef(paused),
    requestBusy = useRef<number | null>(null);
  pausedRef.current = paused;
  useEffect(() => {
    const id = ++generation.current;
    setLocal(null);
    setCloud(null);
    setBusy(false);
    setDetailStatus("Loading chapters and saved progress…");
    invoke<Saved | null>("local_position", { asin: book.asin })
      .then((v) => {
        if (id === generation.current) setLocal(v);
      })
      .catch(() => {});
    void readRef.current();
    const refresh = () => {
      if (pausedRef.current) void readRef.current(true);
    };
    const timer = setInterval(refresh, 30000);
    window.addEventListener("focus", refresh);
    return () => {
      generation.current++;
      clearInterval(timer);
      window.removeEventListener("focus", refresh);
    };
  }, [book.asin]);
  const time = (n: number) =>
    `${Math.floor(n / 3600)}h ${Math.floor(n / 60) % 60}m ${Math.floor(n) % 60}s`;
  const read = async (silent = false) => {
    const id = generation.current;
    if (requestBusy.current === id) return;
    requestBusy.current = id;
    if (!silent) setBusy(true);
    try {
      const v = await invoke<CloudMetadata>("api_position", {
        asin: book.asin,
      });
      if (id !== generation.current) return;
      setCloud(v.position);
      if (v.chapters.length) onChapters(v.chapters);
      setDetailStatus(
        (v.position === null
          ? "Audible position unavailable"
          : `Audible position ${time(v.position)}`) +
          ` · ${v.chapter_details.length} chapters`,
      );
      if (!silent && v.position !== null)
        onStatus("Saved Audible position refreshed");
    } catch (e) {
      if (id === generation.current) {
        setDetailStatus(String(e));
        if (!silent) onStatus(String(e));
      }
    } finally {
      if (requestBusy.current === id) requestBusy.current = null;
      if (id === generation.current) setBusy(false);
    }
  };
  readRef.current = read;
  return (
    <section className="resume-panel">
      <strong>Saved progress</strong>
      <button disabled={busy} onClick={() => read()}>
        {busy ? "Refreshing…" : "Refresh chapters & progress"}
      </button>
      {local && (
        <button
          disabled={busy || !available || local.position > duration}
          onClick={() => onSeek(local.position)}
        >
          Resume on this PC · {time(local.position)}
        </button>
      )}
      {cloud !== null && (
        <button
          disabled={busy || !available || cloud > duration}
          onClick={() => onSeek(cloud)}
        >
          Resume Audible position · {time(cloud)}
        </button>
      )}
      <small role="status">{detailStatus}</small>
      <small>
        {cloud !== null && available && cloud > duration
          ? "This position does not fit the current playback timeline. Resume through the official Audible player."
          : "Playback checkpoints are saved on this PC. Audible’s official player handles account progress."}
      </small>
    </section>
  );
}
