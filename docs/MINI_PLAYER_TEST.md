# Windows mini-player update checks

Run the Windows desktop build. Browser previews use simulated data and cannot verify Audible handoffs.

1. Start an owned book. Check the chapter name above the timeline, including a long title at the minimum window width. Previous/Next should select the neighboring chapter through the official player. Previous is disabled in the first chapter; Next is disabled in the last. The Chapters dropdown still works.
2. Click the cover with the mouse and keyboard. A larger resizable window should open, with the correct title/artwork. X, Close and Escape should close it without stopping audio. Dragging the mini-player title should still work. Switching books or signing out should close the artwork window.
3. Pause desktop playback. Play the same title on mobile to a noticeably different time, pause there, and allow Audible to sync. Press Sync position on desktop. Check its success message, chapter and timeline against mobile, then play. Repeat for a position behind desktop. Repeat with desktop already playing and verify play/pause is preserved.
4. While a check is pending, change chapters or books, or sign out. Its result must not seek the new book, expose the previous account's position or resume audio. Retry on the intended book afterward.
5. Disconnect the network before pressing Sync position. Expect an actionable error and no jump to an old local checkpoint. If Audible has not yet saved mobile progress, it may return the previous spot; wait and retry. No automatic idle seek is implemented.

Native tests cover fresh matching full-book telemetry, valid targets in either direction, rejection outside seekable ranges and player-intent invalidation. Existing account lifecycle tests cover revoked generations. Actual mobile-to-desktop timing still needs this live check.
