/**
 * Transport contract for the development streaming experiment. Separates native
 * IPC from the ordinary-browser comparison route and exposes cleanup for
 * command subscriptions.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Action } from "./playback";
export interface StreamTransport {
  invoke<T>(command: string, args: Record<string, unknown>): Promise<T>;
  commands(
    handler: (action: Action, value: number) => void,
  ): Promise<() => void>;
  label: string;
}
export const nativeStreamTransport: StreamTransport = {
  invoke: (command, args) => invoke(command, args),
  commands: (handler) =>
    listen<{ action: Action; value: number }>("api-player-command", (e) =>
      handler(e.payload.action, e.payload.value),
    ),
  label: "Playback runtime: embedded WebView2",
};
