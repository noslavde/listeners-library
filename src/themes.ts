/**
 * Theme token catalog, resolution and immediate application. The local cache is
 * a non-secret appearance hint; native encrypted settings are authoritative
 * when available.
 */

export type Mode = "dark" | "light";
export type Tokens = {
  bg: string;
  surface: string;
  raised: string;
  control: string;
  input: string;
  text: string;
  muted: string;
  border: string;
  line: string;
  accent: string;
  onAccent: string;
  accentSoft: string;
};
function colors(
  bg: string,
  surface: string,
  raised: string,
  control: string,
  input: string,
  text: string,
  muted: string,
  border: string,
  line: string,
  accent: string,
  onAccent: string,
  accentSoft: string,
): Tokens {
  return {
    bg,
    surface,
    raised,
    control,
    input,
    text,
    muted,
    border,
    line,
    accent,
    onAccent,
    accentSoft,
  };
}
export const palettes = [
  {
    id: "navy-gold",
    name: "Navy & gold",
    description: "Midnight blue · warm gold",
    dark: colors(
      "#11151c",
      "#1b222d",
      "#202935",
      "#27313f",
      "#1d2530",
      "#e9edf2",
      "#adb9ca",
      "#56657a",
      "#303946",
      "#e4ab5f",
      "#20170b",
      "#332b20",
    ),
    light: colors(
      "#f8f6f0",
      "#fffdf8",
      "#efe9dd",
      "#f0eadf",
      "#ffffff",
      "#212c40",
      "#586273",
      "#a9a392",
      "#ddd7c9",
      "#93500b",
      "#ffffff",
      "#f6e6c8",
    ),
  },
  {
    id: "crimson",
    name: "Crimson & champagne",
    description: "Rose red · warm ivory",
    dark: colors(
      "#191215",
      "#261b21",
      "#302128",
      "#37262e",
      "#20171b",
      "#faedf0",
      "#c7afb7",
      "#805461",
      "#49313a",
      "#ff9eaa",
      "#3c0a17",
      "#452430",
    ),
    light: colors(
      "#fff5f2",
      "#fffaf8",
      "#f4e4e0",
      "#f5e7e1",
      "#fffdfc",
      "#3c2026",
      "#77525c",
      "#bd8c99",
      "#e7c9cf",
      "#ab2345",
      "#ffffff",
      "#f8dde2",
    ),
  },
  {
    id: "forest",
    name: "Forest & sage",
    description: "Deep green · soft sage",
    dark: colors(
      "#101a17",
      "#192822",
      "#20332a",
      "#263b32",
      "#14211b",
      "#e9f5ed",
      "#acc7b7",
      "#527765",
      "#304b3c",
      "#8dd9ae",
      "#102a1c",
      "#244334",
    ),
    light: colors(
      "#f2f8ef",
      "#fbfff7",
      "#e9f1e2",
      "#e4efdf",
      "#ffffff",
      "#22372a",
      "#516951",
      "#9ab595",
      "#cbdcc6",
      "#23663f",
      "#ffffff",
      "#d8ecd4",
    ),
  },
  {
    id: "ocean",
    name: "Ocean & ice",
    description: "Ocean blue · cool ice",
    dark: colors(
      "#101923",
      "#192737",
      "#203248",
      "#253a50",
      "#142130",
      "#e6f2ff",
      "#a8c1da",
      "#527393",
      "#304c65",
      "#8bcafa",
      "#102b40",
      "#203f59",
    ),
    light: colors(
      "#f0f7fc",
      "#fbfdff",
      "#e6f0f8",
      "#e0edf7",
      "#ffffff",
      "#173149",
      "#4b667d",
      "#92afc8",
      "#c6d8e6",
      "#195e9e",
      "#ffffff",
      "#d7e9f9",
    ),
  },
  {
    id: "plum",
    name: "Plum & lavender",
    description: "Rich purple · lavender",
    dark: colors(
      "#191423",
      "#251d34",
      "#302541",
      "#382c4b",
      "#20182d",
      "#f3edff",
      "#c4b4d8",
      "#78608e",
      "#49395e",
      "#d1b0ff",
      "#29143f",
      "#3e2b58",
    ),
    light: colors(
      "#f8f3fc",
      "#fffaff",
      "#f1e8f7",
      "#eee3f6",
      "#ffffff",
      "#342343",
      "#6c567a",
      "#b59bc7",
      "#dccbe8",
      "#77429d",
      "#ffffff",
      "#ead9f8",
    ),
  },
  {
    id: "black-green",
    name: "Black & green",
    description: "Carbon black · fresh green",
    dark: colors(
      "#090c0a",
      "#111813",
      "#17221a",
      "#1d2b20",
      "#0e1510",
      "#ecf6ee",
      "#a6bba9",
      "#4c6e51",
      "#2c4430",
      "#91ed8f",
      "#102b11",
      "#213e24",
    ),
    light: colors(
      "#f4f7f1",
      "#ffffff",
      "#edf2e8",
      "#e4eddf",
      "#ffffff",
      "#1b2a1c",
      "#566951",
      "#99b58f",
      "#cfddc9",
      "#2f6b2c",
      "#ffffff",
      "#ddefd6",
    ),
  },
  {
    id: "white-red",
    name: "White & red",
    description: "Porcelain white · cherry red",
    dark: colors(
      "#171719",
      "#232326",
      "#2b2b2f",
      "#323236",
      "#1c1c1f",
      "#faf6f6",
      "#c6b9bb",
      "#766166",
      "#483b3f",
      "#ffa5a6",
      "#3b0c10",
      "#442b30",
    ),
    light: colors(
      "#fffafa",
      "#ffffff",
      "#f7f1f2",
      "#f4eeee",
      "#ffffff",
      "#282326",
      "#6d5b60",
      "#bda0a5",
      "#e6d2d5",
      "#b52635",
      "#ffffff",
      "#fbe1e4",
    ),
  },
] as const;
export const defaultTheme = "navy-gold-dark";
/**
 * Resolves a saved identifier to an available palette/mode with a safe
 * fallback.
 */
export function resolveTheme(id: string) {
  for (const palette of palettes)
    for (const mode of ["dark", "light"] as const)
      if (id === `${palette.id}-${mode}`)
        return { palette, mode, colors: palette[mode] };
  return {
    palette: palettes[0],
    mode: "dark" as Mode,
    colors: palettes[0].dark,
  };
}
const cacheKey = "audible-appearance";
/**
 * Reads the non-secret appearance hint used before native settings finish
 * loading.
 */
export function cachedTheme() {
  try {
    return localStorage.getItem(cacheKey) ?? defaultTheme;
  } catch {
    return defaultTheme;
  }
}
/**
 * Applies resolved CSS tokens to the document and updates the immediate
 * appearance cache.
 */
export function applyTheme(id: string) {
  const theme = resolveTheme(id);
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.colors)) {
    const name = key.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
    root.style.setProperty("--" + name, value);
  }
  root.style.colorScheme = theme.mode;
  root.dataset.theme = `${theme.palette.id}-${theme.mode}`;
  try {
    localStorage.setItem(cacheKey, root.dataset.theme);
  } catch {}
  return theme;
}
