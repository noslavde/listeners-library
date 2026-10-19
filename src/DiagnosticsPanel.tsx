import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { desktop } from "./playback";

export function DiagnosticsPanel() {
  const [preview, setPreview] = useState("");
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  if (!desktop) return null;
  async function review() {
    setBusy(true);
    setStatus("");
    try {
      setPreview(await invoke<string>("diagnostic_report"));
    } catch {
      setStatus(
        "Could not read diagnostics. Your saved account data is unaffected.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function save() {
    setBusy(true);
    setStatus("");
    try {
      const saved = await invoke<boolean>("export_diagnostics", { preview });
      setStatus(
        saved
          ? "Report saved. Nothing was uploaded. Share the JSON file only if you choose."
          : "Save cancelled.",
      );
    } catch {
      setStatus(
        "Could not save this report. Refresh the preview and try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <details className="app-details">
      <summary>Troubleshooting & diagnostic report</summary>
      <p>
        The app keeps up to 128 technical events from each of the last two
        sessions on this PC. No automatic uploads. Reports exclude account
        details, book information, notes, URLs, file paths and raw error
        messages.
      </p>
      <p>
        An unclean shutdown can mean a crash, power loss or a forced stop. Some
        crashes leave no error event. This report helps developers investigate;
        it is not a complete crash dump.
      </p>
      <button disabled={busy} onClick={() => void review()}>
        Review diagnostic report
      </button>
      {preview && (
        <>
          <pre
            aria-label="Diagnostic report preview"
            style={{
              maxHeight: 320,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              overflowWrap: "anywhere",
            }}
          >
            {preview}
          </pre>
          <button disabled={busy} onClick={() => void save()}>
            Save reviewed report…
          </button>
        </>
      )}
      {status && <p role="status">{status}</p>}
    </details>
  );
}
