# Windows mini-player update checks

Run the Windows desktop build. Browser previews use simulated data and cannot verify Audible handoffs.

1. Start an owned book in the middle of a chapter. Previous should first restart that chapter. A second click within three seconds should select the preceding chapter, with further quick clicks continuing backward even before the timeline catches up. After waiting more than three seconds, Previous should restart again. The first chapter can be restarted. Next is disabled in the last chapter. The Chapters dropdown still works.
2. Click the cover with the mouse and keyboard. A larger resizable window should open, with the correct title/artwork. X, Close and Escape should close it without stopping audio. Dragging the mini-player title should still work. Switching books or signing out should close the artwork window.
3. Pause desktop playback. Play the same title on mobile to a noticeably different time, pause there, and allow Audible to sync. Press Sync position on desktop. Check its success message, chapter and timeline against mobile, then play. Repeat for a position behind desktop. Repeat with desktop already playing and verify play/pause is preserved.
4. While a check is pending, change chapters or books, or sign out. Its result must not seek the new book, expose the previous account's position or resume audio. Retry on the intended book afterward.
5. Disconnect the network before pressing Sync position. Expect an actionable error and no jump to an old local checkpoint. If Audible has not yet saved mobile progress, it may return the previous spot; wait and retry. No automatic idle seek is implemented.

6. Play for at least 15 seconds, note the position, pause and normally close the app. Reopen it and listen to the same title. If Audible opens a different spot, History must pause playback and offer the dated PC checkpoint. Restore it and verify the time/chapter. Repeat with an intentional rewind to confirm the newest choice can be earlier in the book. Updates must keep this history.
7. Switch from book A to B and back, using one Listen click each time. The previous player should close; the requested full book should open in the mini player. Test both a cached title and one requiring a library search. Verify remembered sign-in, and separately check private website-session mode if used.
8. In History, restore an older entry while paused and while playing. Switch books or sign out during a restore: it must not apply to another book/account or resume signed-out audio. Logs must disappear from the UI on sign-out and return only for their own account. Returning to the library without choosing a position must not replace the PC checkpoint.

Native tests cover fresh matching full-book telemetry, valid targets in either direction, rejection outside seekable ranges and player-intent invalidation. Existing account lifecycle tests cover revoked generations. Actual mobile-to-desktop timing still needs this live check.
