/**
 * Frontend build/development settings. The strict local port matches Tauri's
 * development URL; production account and playback policy lives in native code.
 */
import { defineConfig } from "vite";
export default defineConfig({
  server: { port: 1420, strictPort: true },
  clearScreen: false,
});
