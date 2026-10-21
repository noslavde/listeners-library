/**
 * Development-only ordinary-browser comparison entry point. Pairs to the local
 * native experiment and uses the same protected-stream engine to distinguish
 * runtime issues from host integration issues.
 */

import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  pairBrowser,
  browserTransport,
  checkBrowserConnection,
} from "./browserTransport";
import { StreamEngine, playbackError } from "./streamEngine";
import { StreamLog, safePlaybackMessage } from "./streamLog";
import { directCapabilities } from "./directPlayback";
import { dispatchDirectCommand } from "./directCommands";
import { ChapterSelect } from "./ChapterSelect";
import type { Book, Snapshot } from "./model";
import "./style.css";

const empty: Snapshot = {
  position: 0,
  duration: 0,
  speed: 1,
  paused: true,
  available: false,
};
function Comparison() {
  const [book, setBook] = useState<Book>(),
    [status, setStatus] = useState("Connecting to the local desktop app…"),
    [diagnostics, setDiagnostics] = useState(""),
    [log, setLog] = useState(""),
    [busy, setBusy] = useState(false),
    [state, setState] = useState(empty),
    [copyStatus, setCopyStatus] = useState("");
  const audio = useRef<HTMLAudioElement>(null),
    engine = useRef<StreamEngine | null>(null),
    trace = useRef<StreamLog | null>(null);
  useEffect(() => {
    let disposed = false,
      checking = false;
    let heartbeat: ReturnType<typeof setInterval> | undefined;
    pairBrowser()
      .then((value) => {
        if (disposed) return;
        const b = value as Book;
        if (!/^[A-Z0-9]{10}$/.test(b.asin) || typeof b.title !== "string")
          throw Error("The comparison did not receive a valid title");
        setBook(b);
        setStatus("Connected · select Load browser playback");
        heartbeat = setInterval(() => {
          if (checking) return;
          checking = true;
          checkBrowserConnection()
            .catch(async () => {
              if (disposed) return;
              clearInterval(heartbeat);
              trace.current?.add(
                "Local comparison connection stopped or expired",
              );
              if (engine.current) {
                await engine.current.destroy();
                engine.current = null;
              }
              setBook(undefined);
              setState(empty);
              setStatus(
                "Comparison connection stopped or expired · create a new link in Tauri",
              );
            })
            .finally(() => {
              checking = false;
            });
        }, 3000);
      })
      .catch((error) => {
        if (!disposed) setStatus(safePlaybackMessage(error));
      });
    return () => {
      disposed = true;
      clearInterval(heartbeat);
      if (engine.current) void engine.current.destroy();
    };
  }, []);
  const load = async () => {
    if (!book || !audio.current) return;
    setBusy(true);
    try {
      if (engine.current) await engine.current.destroy();
      setState(empty);
      setDiagnostics("");
      setCopyStatus("");
      trace.current = new StreamLog(setLog);
      const capabilities = await directCapabilities();
      const capability = capabilities.find(
        (c) => c.keySystem === "com.widevine.alpha",
      );
      trace.current.add(
        "Browser Widevine capability: " +
          (capability?.supported ? "supported" : "unavailable"),
      );
      if (!capability?.supported)
        throw Error(
          "This browser did not expose the required encrypted AAC configuration",
        );
      engine.current = new StreamEngine(
        audio.current,
        book,
        setStatus,
        setDiagnostics,
        (message) => trace.current?.add(message),
        browserTransport(setState),
      );
      await engine.current.load();
    } catch (error) {
      const message = playbackError(error);
      trace.current?.add(message);
      setStatus(message);
      if (engine.current) {
        await engine.current.destroy();
        engine.current = null;
      }
    } finally {
      setBusy(false);
    }
  };
  const stop = async () => {
    if (engine.current) {
      await engine.current.destroy();
      engine.current = null;
    }
    setState(empty);
    setStatus(
      "Browser audio stopped · use Stop browser test in Tauri to close the local connection",
    );
  };
  const time = (n: number) =>
    `${Math.floor(n / 3600)}:${String(Math.floor(n / 60) % 60).padStart(2, "0")}:${String(Math.floor(n) % 60).padStart(2, "0")}`;
  return (
    <main className="browser-comparison">
      <h1>Browser playback comparison</h1>
      <p>
        Use Edge or Chrome and keep Tauri running. This uses the same player and
        account API as the desktop test. Only the playback runtime changes.
      </p>
      <p>
        Account credentials stay in the desktop backend. This temporary
        connection lasts 20 minutes. Playback here does not upload progress or
        write local position checkpoints.
      </p>
      <h2>{book?.title ?? "Connecting…"}</h2>
      <audio
        ref={audio}
        controls
        preload="none"
        aria-label="Browser comparison audio"
      />
      <p role="status">{status}</p>
      <p>{diagnostics}</p>
      <div className="toolbar">
        <button disabled={!book || busy} onClick={load}>
          {busy ? "Loading…" : "Load browser playback"}
        </button>
        <button
          disabled={!state.available}
          onClick={() =>
            dispatchDirectCommand(state.paused ? "play" : "pause", 0)
          }
        >
          {state.paused ? "Play" : "Pause"}
        </button>
        <button
          disabled={!state.available}
          onClick={() => dispatchDirectCommand("seek", state.position - 15)}
        >
          −15s
        </button>
        <button
          disabled={!state.available}
          onClick={() => dispatchDirectCommand("seek", state.position + 15)}
        >
          +15s
        </button>
        <button onClick={() => void stop()}>Stop audio</button>
      </div>
      <p>
        {time(state.position)} / {time(state.duration)}
      </p>
      <input
        aria-label="Browser playback position"
        type="range"
        min={0}
        max={state.duration || 1}
        value={state.position}
        disabled={!state.available}
        onChange={(e) => dispatchDirectCommand("seek", Number(e.target.value))}
      />
      {book && (
        <ChapterSelect
          book={book}
          position={state.position}
          duration={state.duration}
          available={state.available}
          onSeek={(position) => dispatchDirectCommand("seek", position)}
        />
      )}
      <label>
        Speed{" "}
        <select
          aria-label="Browser playback speed"
          value={state.speed}
          onChange={(e) =>
            dispatchDirectCommand("speed", Number(e.target.value))
          }
        >
          {[0.75, 1, 1.25, 1.5, 2, 2.5, 3].map((speed) => (
            <option key={speed} value={speed}>
              {speed}×
            </option>
          ))}
        </select>
      </label>
      <section className="playback-log">
        <h2>Comparison log</h2>
        <textarea
          readOnly
          rows={16}
          aria-label="Browser comparison log"
          value={log}
        />
        <button
          disabled={!log}
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(log);
              setCopyStatus("Copied comparison log");
            } catch {
              setCopyStatus("Select the log text and copy it manually");
            }
          }}
        >
          Copy comparison log
        </button>
        <p>{copyStatus}</p>
      </section>
    </main>
  );
}
createRoot(document.getElementById("browser-test-root")!).render(
  <Comparison />,
);
