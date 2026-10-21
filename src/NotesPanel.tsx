/**
 * Compatibility entry point for the saved-moments feature. Re-exports
 * HighlightsPanel so existing callers use the richer bookmarks/notes UI without
 * a second implementation.
 */

export { HighlightsPanel as NotesPanel } from "./HighlightsPanel";
