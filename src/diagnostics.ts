import { invoke } from "@tauri-apps/api/core";
import { desktop } from "./playback";

type Code = "react-error" | "script-error" | "promise-rejected";
export function reportUiError(code: Code) {
  if (desktop) void invoke("record_ui_error", { code }).catch(() => {});
}
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
