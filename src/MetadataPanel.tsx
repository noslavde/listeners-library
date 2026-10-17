import { protectionName } from "./platform";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "./playback";
import type { Book } from "./model";
import { createAccountSync } from "./accountSync";
type Connection = {
  connected: boolean;
  stable_profile?: boolean;
  cleanup_required?: boolean;
  phase?: string;
  message?: string;
  version?: string;
  build?: string;
  diagnostics?: string[];
};
const active = (phase?: string) =>
  ["signing-in", "restoring", "registering", "saving", "signing-out"].includes(
    phase ?? "",
  );
export function MetadataPanel({
  onLibrary,
  onStatus,
  onConnection,
}: {
  onLibrary: (books: Book[]) => void;
  onStatus: (message: string) => void;
  onConnection: (connected: boolean) => void;
}) {
  const [connection, setConnection] = useState<Connection>({
      connected: false,
    }),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [syncing, setSyncing] = useState(false),
    [syncError, setSyncError] = useState("");
  const library = useRef(onLibrary),
    syncNow = useRef<() => Promise<void>>(async () => {});
  library.current = onLibrary;
  const connectionChanged = useRef(onConnection);
  connectionChanged.current = onConnection;
  const waiting = useRef(false),
    report = useRef(onStatus),
    refreshNow = useRef<() => Promise<void>>(async () => {});
  report.current = onStatus;
  useEffect(() => {
    if (!desktop) return;
    let dead = false,
      stop: (() => void) | undefined,
      version = 0,
      lastMessage = "";
    const sync = createAccountSync({
      load: () => invoke<Book[]>("api_sync"),
      publish: (books) => library.current(books),
      status: (message) => report.current(message),
      error: setSyncError,
      busy: setSyncing,
    });
    syncNow.current = sync.sync;
    const refresh = async () => {
      const id = ++version;
      try {
        const value = await invoke<Connection>("api_connection");
        if (dead || id !== version) return;
        setConnection(value);
        connectionChanged.current(value.connected);
        waiting.current = active(value.phase);
        setError("");
        if (value.message && value.message !== lastMessage) {
          lastMessage = value.message;
          report.current(value.message);
        }
        sync.observe(value.connected);
      } catch (e) {
        if (!dead && id === version) setError(String(e));
      }
    };
    refreshNow.current = refresh;
    // Subscribe before reading, then recover missed events while the approved
    // exchange is active and whenever the library window regains focus.
    void listen("api-account-changed", () => void refresh())
      .then((f) => {
        if (dead) f();
        else {
          stop = f;
          void refresh();
        }
      })
      .catch((e) => {
        if (!dead) {
          setError(String(e));
          void refresh();
        }
      });
    const focused = () => void refresh();
    window.addEventListener("focus", focused);
    let stopSignedOut: (() => void) | undefined;
    void listen("account-signed-out", () => {
      version++;
      sync.observe(false);
      setConnection({ connected: false, phase: "signing-out" });
      connectionChanged.current(false);
      waiting.current = true;
    }).then((f) => {
      if (dead) f();
      else stopSignedOut = f;
    });
    const timer = setInterval(() => {
      if (waiting.current) void refresh();
    }, 1000);
    return () => {
      dead = true;
      version++;
      sync.dispose();
      stop?.();
      stopSignedOut?.();
      clearInterval(timer);
      window.removeEventListener("focus", focused);
    };
  }, []);
  const run = async (f: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await f();
    } catch (e) {
      waiting.current = false;
      setError(String(e));
      onStatus(String(e));
    } finally {
      setBusy(false);
    }
  };
  if (!desktop) return null;
  const pending = active(connection.phase),
    connected = connection.connected;
  const disconnect = () =>
    run(async () => {
      try {
        await invoke("api_disconnect");
        onStatus("Signed out. Playback stopped and your library is hidden.");
      } finally {
        await refreshNow.current();
      }
    });
  const diagnostic = [
    `Listener’s Library ${connection.version ?? "unknown"} (${connection.build ?? "unknown"})`,
    `Account: ${connected ? "connected" : "disconnected"} · Stage: ${connection.phase ?? "unknown"}`,
    connection.message ?? "",
    ...(connection.diagnostics ?? []),
  ].join("\n");
  return (
    <section className="account-bar">
      <strong>
        {connected
          ? "Library account connected"
          : pending
            ? "Connecting Audible…"
            : "Connect Audible"}
      </strong>
      {!connected && (
        <button
          disabled={busy || syncing || pending}
          onClick={() =>
            run(async () => {
              waiting.current = true;
              await invoke("api_login", { profile: "android" });
              await refreshNow.current();
            })
          }
        >
          {pending ? "Connecting…" : "Sign in"}
        </button>
      )}
      <button
        disabled={!connected || busy || syncing}
        onClick={() => void syncNow.current()}
      >
        {syncing ? "Syncing…" : busy ? "Working…" : "Sync library"}
      </button>
      <details>
        <summary>Account</summary>
        <p>
          Account tokens are encrypted using {protectionName}. Library data and
          notes use an encrypted database.
        </p>
        <button
          disabled={
            (!connected && !pending && !connection.cleanup_required) || busy
          }
          onClick={disconnect}
        >
          {connection.cleanup_required
            ? "Retry sign-out"
            : pending && !connected
              ? "Cancel sign-in"
              : "Sign out"}
        </button>
      </details>
      <details>
        <summary>Sign-in diagnostic</summary>
        <p>
          Contains connection stages only. No account details or credentials.
        </p>
        <pre style={{ whiteSpace: "pre-wrap", userSelect: "text" }}>
          {diagnostic}
        </pre>
        <button
          onClick={() =>
            run(async () => {
              await refreshNow.current();
            })
          }
        >
          Refresh connection
        </button>
      </details>
      {(error ||
        connection.phase === "failed" ||
        connection.phase === "cancelled" ||
        connection.cleanup_required) && (
        <p role="status">{error || connection.message}</p>
      )}
      {pending && <p role="status">{connection.message}</p>}
      {connected && (
        <p>
          Library connected. Audible may separately request website verification
          for playback.
        </p>
      )}
      {connected && connection.stable_profile === false && (
        <p role="status">
          This sign-in did not include a stable account identity. A later
          sign-in may show a new local profile. Export important notes before
          signing out.
        </p>
      )}
      {syncError && <p role="status">{syncError}</p>}
    </section>
  );
}
