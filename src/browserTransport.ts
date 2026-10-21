/**
 * Development-only same-origin bridge to the local browser comparison server.
 * Consumes a one-use pairing ticket from the fragment, removes it from the URL
 * and rejects redirects/non-JSON responses.
 */

import type { Snapshot } from "./model";
import type { StreamTransport } from "./streamTransport";
import { safePlaybackMessage } from "./streamLog";

async function request(operation: Record<string, unknown>) {
  const response = await fetch("/api/call", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    mode: "same-origin",
    redirect: "error",
    cache: "no-store",
    body: JSON.stringify(operation),
  });
  if (!response.headers.get("content-type")?.includes("application/json"))
    throw Error(
      "Local comparison connection unavailable; create a new test link in Tauri",
    );
  const value = (await response.json()) as {
    ok: boolean;
    result?: unknown;
    error?: string;
  };
  if (!response.ok || !value.ok)
    throw Error(
      safePlaybackMessage(value.error ?? "Local comparison request failed"),
    );
  return value.result;
}
/**
 * Consumes a one-use pairing ticket from the fragment, removes it from browser
 * history, then requests the paired book context.
 */
export async function pairBrowser() {
  const ticket = new URLSearchParams(location.hash.slice(1)).get("pair");
  history.replaceState(null, "", location.pathname);
  if (ticket) {
    const response = await fetch("/pair", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      mode: "same-origin",
      redirect: "error",
      cache: "no-store",
      body: JSON.stringify({ ticket }),
    });
    if (!response.ok)
      throw Error(
        "Pairing link used or expired; create a new browser test link in Tauri",
      );
  }
  return request({ operation: "book" });
}
/**
 * Checks that the same-origin comparison session still accepts paired requests.
 */
export async function checkBrowserConnection() {
  await request({ operation: "book" });
}
/**
 * Adapts local comparison HTTP operations and commands to StreamTransport;
 * never forwards native account credentials into the browser.
 */
export function browserTransport(
  onState: (state: Snapshot) => void,
): StreamTransport {
  const browser = /Edg\//.test(navigator.userAgent)
    ? "Edge"
    : /Chrome\//.test(navigator.userAgent)
      ? "Chrome"
      : "other browser";
  return {
    label: `Playback runtime: ${browser} · ordinary browser comparison`,
    commands: async () => () => {},
    invoke: async <T>(
      command: string,
      args: Record<string, unknown>,
    ): Promise<T> => {
      if (command === "api_stream_state") {
        onState(args.state as Snapshot);
        return undefined as T;
      }
      const operations: Record<string, string> = {
        api_stream_start: "start",
        api_stream_fetch: "fetch",
        api_stream_license: "license",
        api_stream_end: "end",
      };
      const operation = operations[command];
      if (!operation)
        throw Error("Operation is not available in the browser comparison");
      return (await request({ ...args, operation })) as T;
    },
  };
}
