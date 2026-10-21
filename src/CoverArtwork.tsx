/**
 * Dedicated cover-artwork window. Reads the native-selected safe image and
 * closes independently of playback; no signed-in website or account token is
 * rendered here.
 */

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { safeCover } from "./Cover";

/**
 * Loads native-selected artwork for this dedicated window and disposes its
 * event/keyboard subscriptions on close.
 */
export function CoverArtwork() {
  const [art, setArt] = useState<{ title: string; url: string }>();
  const [error, setError] = useState("");
  const close = () =>
    void invoke("close_cover").catch((e) => setError(String(e)));
  useEffect(() => {
    let dead = false;
    void invoke<{ title: string; url: string }>("current_cover")
      .then((value) => {
        if (!dead) {
          if (safeCover(value.url)) setArt(value);
          else setError("Cover artwork is unavailable.");
        }
      })
      .catch((e) => {
        if (!dead) setError(String(e));
      });
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", key);
    return () => {
      dead = true;
      window.removeEventListener("keydown", key);
    };
  }, []);
  return (
    <section className="cover-artwork" aria-label="Book cover artwork">
      <div className="cover-artwork-heading">
        <strong>{art?.title ?? "Cover artwork"}</strong>
        <button autoFocus onClick={close} aria-label="Close cover artwork">
          Close ×
        </button>
      </div>
      {error ? (
        <p role="status">{error}</p>
      ) : art ? (
        <img
          src={art.url}
          alt={`Cover of ${art.title}`}
          referrerPolicy="no-referrer"
          onError={() =>
            setError(
              "Cover artwork could not be loaded. Check your connection and reopen it.",
            )
          }
        />
      ) : (
        <p role="status">Loading artwork…</p>
      )}
    </section>
  );
}
