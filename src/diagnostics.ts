/**
 * Frontend error reporting through fixed diagnostic codes. Raw exceptions,
 * URLs, stack traces and user content are deliberately not forwarded to the
 * diagnostic journal.
 */

import { invoke } from "@tauri-apps/api/core";
import { desktop } from "./playback";

type Code = "react-error" | "script-error" | "promise-rejected";
/**
 * Forwards only a fixed diagnostic code; never pass raw errors, stack traces,
 * book names or note text.
 */
export function reportUiError(code: Code) {
  if (desktop) void invoke("record_ui_error", { code }).catch(() => {});
}
/**
 * Registers global failure handlers that translate exceptions/rejections into
 * allowlisted codes, excluding their payloads.
 */
export function installErrorReporting() {
  // Never pass ErrorEvent, Error, rejection reasons or stack traces to native.
  const error = () => reportUiError("script-error");
  const rejection = () => reportUiError("promise-rejected");
  window.addEventListener("error", error);
  window.addEventListener("unhandledrejection", rejection);
  return () => {
    window.removeEventListener("error", error);
    window.removeEventListener("unhandledrejection", rejection);
  };
}
