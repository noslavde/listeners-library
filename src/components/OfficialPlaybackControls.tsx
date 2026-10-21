/**
 * Manual official-window visibility and website-session recovery actions.
 * Library API connection and website sign-in are separate states, so resetting
 * one does not silently imply resetting both.
 */

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { desktop } from "../playback";

/**
 * Manual website recovery controls for cases where the official player needs
 * verification. Forgetting website sign-in does not itself disconnect the
 * library API account.
 */
export function OfficialControls({
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
          Sign-in is remembered on this PC. Open Audible if playback needs
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
