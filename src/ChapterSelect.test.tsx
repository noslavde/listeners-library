/**
 * Regression tests covering chapter availability. Uses synthetic fixtures;
 * these checks do not verify real account sign-in, audio output or mobile
 * synchronization.
 */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, it, expect } from "vitest";
import { ChapterSelect } from "./ChapterSelect";
import { demoBooks } from "./model";
describe("chapter availability", () => {
  it("keeps metadata chapters selectable outside buffered duration and before media is ready", () => {
    const html = renderToStaticMarkup(
      <ChapterSelect
        book={demoBooks[0]}
        position={0}
        duration={100}
        available={false}
        seekable={[{ start: 0, end: 100 }]}
        onSeek={() => {}}
      />,
    );
    expect(html).not.toMatch(/<select[^>]*disabled/);
    expect(html).toMatch(/<option value="1800">/);
    expect(html).not.toMatch(/<option[^>]*value="1800"[^>]*disabled/);
  });
});
