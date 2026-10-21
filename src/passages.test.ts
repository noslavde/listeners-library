import { describe, expect, it } from "vitest";
import { excerpt, parseStamp, passageError, passageOnly } from "./passages";
describe("saved passages", () => {
  it("rejects ambiguous or malformed timestamps", () => {
    expect(parseStamp("2:04:09")).toBe(7449);
    for (const input of [
      "2:99:00",
      "-1:00:00",
      "1:01",
      "NaN",
      "1:00:00<script>",
    ])
      expect(parseStamp(input)).toBeUndefined();
  });
  it("captures the preceding thirty seconds and clamps at book boundaries", () => {
    expect(excerpt(78.9, 100)).toEqual([48, 78]);
    expect(excerpt(12, 100)).toEqual([0, 12]);
    expect(excerpt(0, 15)).toEqual([0, 15]);
  });
  it("enforces clip ranges and UTF-8 text limits", () => {
    const p = {
      kind: "clip" as const,
      position: 30,
      end_position: 60,
      text: "",
    };
    expect(passageError(p, 100)).toBe("");
    expect(passageError({ ...p, end_position: 30 }, 100)).not.toBe("");
    expect(passageError({ ...p, end_position: 400 }, 500)).not.toBe("");
    expect(passageError({ ...p, text: "🙂".repeat(2600) }, 100)).not.toBe("");
    expect(passageOnly({ ...p, id: 9 } as typeof p)).toEqual(p);
  });
});
