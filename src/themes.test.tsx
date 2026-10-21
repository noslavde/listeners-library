/**
 * Regression tests covering theme palettes; Explore selection. Uses synthetic
 * fixtures; these checks do not verify real account sign-in, audio output or
 * mobile synchronization.
 */

import { describe, it, expect } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { palettes, resolveTheme, defaultTheme } from "./themes";
import { ExploreNav } from "./ExploreNav";
function luminance(hex: string) {
  const rgb = hex
    .slice(1)
    .match(/../g)!
    .map((v) => parseInt(v, 16) / 255)
    .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
}
function contrast(a: string, b: string) {
  const x = luminance(a),
    y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
describe("theme palettes", () => {
  it("provides both brightness modes for every requested palette and falls back safely", () => {
    expect(palettes).toHaveLength(7);
    for (const p of palettes)
      for (const mode of ["dark", "light"] as const) {
        const theme = resolveTheme(`${p.id}-${mode}`);
        expect(theme.palette.id).toBe(p.id);
        expect(theme.mode).toBe(mode);
      }
    expect(
      resolveTheme("unknown").palette.id + "-" + resolveTheme("unknown").mode,
    ).toBe(defaultTheme);
  });
  for (const p of palettes)
    for (const mode of ["dark", "light"] as const)
      it(`${p.name} ${mode} keeps text and selected controls readable`, () => {
        const c = p[mode];
        for (const bg of [
          c.bg,
          c.surface,
          c.control,
          c.raised,
          c.input,
          c.accentSoft,
        ]) {
          expect(
            contrast(c.text, bg),
            "main text on " + bg,
          ).toBeGreaterThanOrEqual(4.5);
          expect(
            contrast(c.muted, bg),
            "secondary text on " + bg,
          ).toBeGreaterThanOrEqual(4.5);
        }
        for (const bg of [c.bg, c.surface, c.accentSoft])
          expect(
            contrast(c.accent, bg),
            "accent text on " + bg,
          ).toBeGreaterThanOrEqual(4.5);
        expect(
          contrast(c.onAccent, c.accent),
          "primary button text",
        ).toBeGreaterThanOrEqual(4.5);
      });
});
describe("Explore selection", () => {
  it("marks only All titles selected initially", () => {
    const html = renderToStaticMarkup(
      <ExploreNav
        series={["Northern Skies", "Other"]}
        selected=""
        onSelect={() => {}}
      />,
    );
    expect(html.match(/aria-pressed="true"/g)).toHaveLength(1);
    expect(html).toContain(
      'aria-pressed="true"><span aria-hidden="true">● </span>All titles',
    );
  });
  it("moves the selected marker to the series chosen anywhere in the app", () => {
    const html = renderToStaticMarkup(
      <ExploreNav
        series={["Northern Skies", "Other"]}
        selected="Northern Skies"
        onSelect={() => {}}
      />,
    );
    expect(html.match(/aria-pressed="true"/g)).toHaveLength(1);
    expect(html).toContain('aria-pressed="false">All titles');
    expect(html).toContain(
      'aria-pressed="true"><span aria-hidden="true">● </span>Northern Skies',
    );
  });
});
