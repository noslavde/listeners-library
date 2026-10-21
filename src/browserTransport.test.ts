/**
 * Regression tests covering browser comparison transport. Uses synthetic
 * fixtures; these checks do not verify real account sign-in, audio output or
 * mobile synchronization.
 */

import { afterEach, describe, it, expect, vi } from "vitest";
import { browserTransport, pairBrowser } from "./browserTransport";
import type { Snapshot } from "./model";
afterEach(() => {
  vi.unstubAllGlobals();
});
function setup() {
  const fetch = vi.fn(
    async () =>
      new Response(
        JSON.stringify({
          ok: true,
          result: { asin: "B012345678", title: "Test" },
        }),
        { headers: { "content-type": "application/json" } },
      ),
  );
  vi.stubGlobal("fetch", fetch);
  vi.stubGlobal("location", { hash: "#pair=local-ticket", pathname: "/" });
  vi.stubGlobal("history", { replaceState: vi.fn() });
  vi.stubGlobal("navigator", { userAgent: "Chrome/155 Edg/155" });
  return fetch;
}
describe("browser comparison transport", () => {
  it("consumes the fragment before pairing and uses same-origin HTTP-only-cookie requests", async () => {
    const fetch = setup();
    await pairBrowser();
    expect(history.replaceState).toHaveBeenCalledWith(null, "", "/");
    expect(
      vi.mocked(history.replaceState).mock.invocationCallOrder[0],
    ).toBeLessThan(fetch.mock.invocationCallOrder[0]);
    expect(fetch).toHaveBeenNthCalledWith(
      1,
      "/pair",
      expect.objectContaining({
        credentials: "same-origin",
        mode: "same-origin",
        redirect: "error",
        body: JSON.stringify({ ticket: "local-ticket" }),
      }),
    );
    expect(fetch).toHaveBeenNthCalledWith(
      2,
      "/api/call",
      expect.objectContaining({ body: JSON.stringify({ operation: "book" }) }),
    );
  });
  it("keeps telemetry in the browser and allows only playback commands across the bridge", async () => {
    const fetch = setup();
    const state = vi.fn();
    const transport = browserTransport(state);
    expect(transport.label).toContain("Edge");
    await transport.invoke("api_stream_state", {
      id: "test",
      state: { position: 1 } as Snapshot,
    });
    expect(state).toHaveBeenCalledWith({ position: 1 });
    expect(fetch).not.toHaveBeenCalled();
    await expect(transport.invoke("api_login", {})).rejects.toThrow(
      "not available",
    );
    expect(fetch).not.toHaveBeenCalled();
    await transport.invoke("api_stream_license", {
      id: "test",
      challenge: [1, 2, 3],
    });
    expect(fetch).toHaveBeenCalledWith(
      "/api/call",
      expect.objectContaining({
        body: JSON.stringify({
          id: "test",
          challenge: [1, 2, 3],
          operation: "license",
        }),
      }),
    );
  });
  it("preserves sanitized backend rejections and gives pairing-expiry errors", async () => {
    const fetch = setup();
    fetch.mockImplementation(
      async () =>
        new Response(
          JSON.stringify({
            ok: false,
            error: "Platform license HTTP 400: untrusted client",
          }),
          { status: 400, headers: { "content-type": "application/json" } },
        ),
    );
    await expect(
      browserTransport(() => {}).invoke("api_stream_license", {
        id: "test",
        challenge: [1],
      }),
    ).rejects.toThrow("HTTP 400");
    await expect(pairBrowser()).rejects.toThrow("used or expired");
  });
});
