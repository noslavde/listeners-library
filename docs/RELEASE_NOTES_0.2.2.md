# Listener’s Library 0.2.2 beta — Windows mini player

- The current chapter is highlighted directly above the timeline, with previous/next chapter buttons. The existing Chapters dropdown is retained.
- **Sync position** fetches Audible's account position for the playing book and requests that position in its official player, preserving play/pause. The app checks the timeline before reporting success. This is a manual handoff in either direction; Audible may take time to save mobile progress. There is no automatic seek after idle.
- Clicking the cover opens a larger, resizable artwork window. Escape, Close or the window's X dismisses it while playback continues.
- Sync requests are canceled when the account, selected book or playback controls change. Cover windows close when switching books, closing the mini player or signing out. Account tokens remain in the native backend; the artwork window receives only a title and an approved public image URL.

Windows-only test build. Mac experimentation remains separate. See [mini-player checks](MINI_PLAYER_TEST.md) for the live handoff test and [getting started](TESTER_GUIDE.md) for usage. Actual mobile-to-desktop progress timing requires live testing before publication.
