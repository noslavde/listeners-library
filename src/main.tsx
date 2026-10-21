/**
 * React entry point for main, mini-player and cover-artwork windows. Shared
 * error reporting and theme initialization run before rendering; business state
 * belongs in App and feature modules.
 */

// Foundation styles must precede feature styles imported by App.
import "./style.css";
import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { MiniPlayer } from "./MiniPlayer";
import { CoverArtwork } from "./CoverArtwork";
import { ErrorBoundary } from "./ErrorBoundary";
import { ThemeManager } from "./ThemePicker";
import { applyTheme, cachedTheme } from "./themes";
import { installErrorReporting } from "./diagnostics";

applyTheme(cachedTheme());
installErrorReporting();
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      {new URLSearchParams(location.search).get("cover") === "1" ? (
        <CoverArtwork />
      ) : (
        <>
          <ThemeManager />
          {new URLSearchParams(location.search).get("mini") === "1" ? (
            <MiniPlayer />
          ) : (
            <App />
          )}
        </>
      )}
    </ErrorBoundary>
  </React.StrictMode>,
);
