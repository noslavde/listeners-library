/**
 * Native listening preferences, sleep-timer status and their controls. The hook
 * subscribes to settings changes; the backend owns validation, persistence and
 * actual timer/volume behavior.
 */

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "./playback";
import type { Book, Snapshot } from "./model";
import { chapterInfo } from "./playerInfo";
export type Preferences = {
  speed: number;
  volume: number;
  rewindSeconds: number;
  pauseAfterChapter: boolean;
  progressMode: "chapter" | "book";
  librarySort: import("./libraryView").Sort;
  theme: string;
};
type Sleep = {
  mode: "time" | "chapter";
  remainingSeconds: number | null;
  chapterEnd: number | null;
};
type ListeningStatus = { preferences: Preferences; sleep: Sleep | null };
const defaults: Preferences = {
  speed: 1,
  volume: 1,
  rewindSeconds: 5,
  pauseAfterChapter: false,
  progressMode: "chapter",
  librarySort: "title",
  theme: "navy-gold-dark",
};
export const speeds = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3];
/**
 * Reads native settings/sleep status and subscribes to changes. Returns native
 * update operations; callers should surface its readiness/error state.
 */
export function useListening() {
  const [state, setState] = useState<ListeningStatus>({
      preferences: defaults,
      sleep: null,
    }),
    [ready, setReady] = useState(false),
    [error, setError] = useState("");
  useEffect(() => {
    if (!desktop) return;
    let dead = false,
      stop: (() => void) | undefined;
    void (async () => {
      try {
        stop = await listen<ListeningStatus>("listening-settings", (e) => {
          if (!dead) {
            setState(e.payload);
            setReady(true);
          }
        });
        if (dead) {
          stop();
          return;
        }
        const value = await invoke<ListeningStatus>("listening_status");
        if (!dead) {
          setState(value);
          setReady(true);
        }
      } catch (e) {
        if (!dead) setError(String(e));
      }
    })();
    return () => {
      dead = true;
      stop?.();
    };
  }, []);
  const run = async (command: string, args: Record<string, unknown>) => {
    try {
      await invoke(command, args);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  };
  return {
    ...state,
    ready,
    error,
    update: (patch: Partial<Preferences>) =>
      run("update_listening_preferences", { patch }),
    timer: (mode: string, minutes?: number) =>
      run("set_sleep_timer", { mode, minutes: minutes ?? null }),
  };
}
/**
 * Formats elapsed seconds for compact controls; shared display helper, not a
 * timestamp parser.
 */
export function clock(seconds: number) {
  const n = Math.max(0, Math.ceil(seconds));
  return n >= 3600
    ? `${Math.floor(n / 3600)}:${String(Math.floor(n / 60) % 60).padStart(2, "0")}:${String(n % 60).padStart(2, "0")}`
    : `${Math.floor(n / 60)}:${String(n % 60).padStart(2, "0")}`;
}
/**
 * Lets the listener configure or cancel a native time/chapter sleep timer.
 */
export function SleepControls({
  settings,
  available,
  chapterRemaining,
}: {
  settings: ReturnType<typeof useListening>;
  available: boolean;
  chapterRemaining?: number;
}) {
  const sleep = settings.sleep;
  return (
    <div className="sleep-controls">
      <label>
        Sleep{" "}
        <select
          aria-label="Sleep timer"
          disabled={!settings.ready}
          value={sleep?.mode === "chapter" ? "chapter" : sleep ? "time" : "off"}
          onChange={(e) => {
            const v = e.target.value;
            if (v === "off" || v === "chapter") void settings.timer(v);
            else if (v !== "time") void settings.timer("time", Number(v));
          }}
        >
          <option value="off">Off</option>
          {sleep?.mode === "time" && (
            <option value="time">Timer running</option>
          )}
          <option value="15" disabled={!available}>
            15 minutes
          </option>
          <option value="30" disabled={!available}>
            30 minutes
          </option>
          <option value="60" disabled={!available}>
            60 minutes
          </option>
          <option
            value="chapter"
            disabled={!available || chapterRemaining === undefined}
          >
            End of chapter
          </option>
        </select>
      </label>
      {sleep && (
        <>
          <span role="timer">
            {sleep.mode === "time"
              ? clock(sleep.remainingSeconds ?? 0)
              : "Chapter end" +
                (chapterRemaining !== undefined
                  ? " · " + clock(chapterRemaining)
                  : "")}
          </span>
          {sleep.mode === "time" && (
            <button onClick={() => void settings.timer("extend")}>
              +15 min
            </button>
          )}
          <button
            aria-label="Cancel sleep timer"
            onClick={() => void settings.timer("off")}
          >
            Cancel
          </button>
        </>
      )}
    </div>
  );
}
/**
 * Edits listening behavior such as pause-after-chapter and rewind-after-pause
 * through validated native preferences.
 */
export function PreferenceControls({
  settings,
}: {
  settings: ReturnType<typeof useListening>;
}) {
  return (
    <div className="preference-controls">
      <label>
        Remembered speed{" "}
        <select
          aria-label="Remembered playback speed"
          value={settings.preferences.speed}
          disabled={!settings.ready}
          onChange={(e) =>
            void settings.update({ speed: Number(e.target.value) })
          }
        >
          {[...new Set([...speeds, settings.preferences.speed])]
            .sort((a, b) => a - b)
            .map((n) => (
              <option key={n} value={n}>
                {n}×
              </option>
            ))}
        </select>
      </label>
      <label>
        <input
          type="checkbox"
          checked={settings.preferences.pauseAfterChapter}
          disabled={!settings.ready}
          onChange={(e) =>
            void settings.update({ pauseAfterChapter: e.target.checked })
          }
        />{" "}
        Pause after every chapter
      </label>
      <label>
        After a pause over 60s, rewind{" "}
        <select
          aria-label="Rewind after a long pause"
          disabled={!settings.ready}
          value={settings.preferences.rewindSeconds}
          onChange={(e) =>
            void settings.update({ rewindSeconds: Number(e.target.value) })
          }
        >
          {[
            ...new Set([
              0,
              5,
              10,
              15,
              20,
              30,
              settings.preferences.rewindSeconds,
            ]),
          ]
            .sort((a, b) => a - b)
            .map((n) => (
              <option key={n} value={n}>
                {n}s
              </option>
            ))}
        </select>
      </label>
      <p>Listening preferences are saved on this PC.</p>
    </div>
  );
}
/**
 * Combines preferences and sleep actions for the main playback-help surface.
 */
export function ListeningControls({
  book,
  snapshot,
}: {
  book?: Book;
  snapshot: Snapshot;
}) {
  const settings = useListening();
  if (!desktop) return null;
  const chapterRemaining =
    settings.sleep?.mode === "chapter" && settings.sleep.chapterEnd !== null
      ? Math.max(0, settings.sleep.chapterEnd - snapshot.position) /
        Math.max(0.5, snapshot.speed)
      : chapterInfo(book, snapshot)?.remaining;
  return (
    <section>
      <SleepControls
        settings={settings}
        available={snapshot.available}
        chapterRemaining={chapterRemaining}
      />
      <PreferenceControls settings={settings} />
      {settings.error && <p role="status">{settings.error}</p>}
    </section>
  );
}
