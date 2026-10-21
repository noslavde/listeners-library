/**
 * Regression tests covering official playback routing. Uses synthetic fixtures;
 * these checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));
vi.hoisted(() => vi.stubGlobal("window", {}));
import { invoke } from "@tauri-apps/api/core";
import { OfficialAdapter } from "./playback";
import { demoBooks } from "./model";

describe("official playback routing", () => {
  beforeEach(() => vi.mocked(invoke).mockClear());
  it("opens the official player and sends its controls through the native bridge", async () => {
    const adapter = new OfficialAdapter();
    await adapter.open({ ...demoBooks[0], asin: "B012345678" });
    await adapter.command("play");
    expect(invoke).toHaveBeenNthCalledWith(1, "open_player", {
      asin: "B012345678",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "player_command", {
      action: "play",
      value: 0,
    });
  });
  it("routes chapter selection through the official chapter list rather than an unloaded media seek", async () => {
    await new OfficialAdapter().chapter(3600, "Chapter 3");
    expect(invoke).toHaveBeenCalledWith("select_chapter", {
      position: 3600,
      title: "Chapter 3",
    });
  });
});
