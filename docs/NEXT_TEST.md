# Next test: choose a book and listen in the mini player

Restart the Tauri development app to load both the frontend and native changes.

Search-result launch update: a selected title's real Listen now popup can now navigate the existing signed-in window directly, without relying solely on a native popup event. Only the exact HTTPS Audible player URL requested by the page for that ASIN is accepted, after its control has been clicked; samples and unrelated popups are left alone. The handler is restored when the attempt ends. This addresses a possible popup blocker gap; the cause of the reported live failure is not yet confirmed. Test a new title requiring search. The safe launch diagnostic adds a `popupRedirects` count.

1. Confirm your cached library is visible. Use **Sign in** and **Sync library** if needed; remembered sign-in should be reused. Audible may still request verification.
2. Choose **Listen in mini player** on a previously opened title. When full-book playback starts, the borderless mini player should open automatically and both the library and official window should hide. A paused title receives one Play request; a sample must never trigger the handoff.
3. Try play/pause, 15-second skips, speed, chapters and seeking. Confirm audio continues while the official window is hidden.
4. Choose **Back to app ×**. The library should return while audio continues, and the mini player should stay closed. **Open mini player** should bring it back. Test Alt+F4 from the mini player too.
5. Return to the library and choose another title, including a new title which requires an Audible library search. Search and highlighting remain enabled. Once that title actually starts, the same automatic mini-player handoff should run. If Listen now automation fails, the selected book remains highlighted for manual selection.
6. Pause and choose **Listen in mini player** for the same book again. It should resume without reloading the title and return to the mini player once playing.

**Playback help & options** on the main screen holds the official-window buttons, resume tools, advanced playback options and development inspection. Mini-player **Help** can reveal the official player. These recovery controls remain available if Audible needs a manual click.

If launching fails, report the visible status and whether the title was already remembered or needed a library search. In development, **Inspect Audible window** and `window.__audibleLaunchDebug` provide bounded launch counts/flags. Share only the relevant button/row markup; do not share cookies, session headers or an entire page dump.

Automated checks: 57 frontend tests now include new-document search results, delayed popup requests after in-place search, restoring the popup handler, and refusing unrelated or unsafe popup URLs. The previous 36 native tests cover, among other behavior, unavailable media, samples, wrong titles, a single Play request, one-time mini opening and cancellation. Automated checks do not establish live Audible playback or window behavior.

Notes, series browsing, chapter percentage and speed-aware finish time remain available. Advanced pause/rewind behavior and cross-device progress still need live verification. Automatic cloud uploads remain off. Historical standalone-streaming investigations are retained in the dedicated research documents; they are outside this test flow.

## Mini-player features to test

- **Sleep:** choose 15, 30 or 60 minutes, check the countdown, add 15 minutes, then cancel. The countdown uses elapsed time even while playback is paused. It stays active when the mini player closes or the library is hidden. Choosing a different book, signing out, or restarting the app clears it.
- **End of chapter:** start near the end of a chapter and choose this option. It should pause once at the captured chapter boundary and clear the timer. Chapter metadata must be available. This differs from **Pause after every chapter**, which stays enabled until switched off.
- **Preferences:** change speed, rewind amount and pause-after-chapter in **Settings & help**. Restart Tauri and start a book again; those choices should return. The speed is a shared listening preference, not yet a separate setting for each book.
- **Progress:** click the Chapter/Book progress label to switch the timeline. The choice should survive a restart. Chapter time remaining and finish time account for playback speed; the paused finish label assumes playback resumes now.
- **Cover art:** sync the library for books cached before this update. Public Amazon image URLs are displayed without a referrer; books without supported artwork retain an initial tile.

The latest checks pass: frontend production build, 76 frontend tests and 43 native tests. The new live controls still need account testing. Sleep timers are intentionally session-only; listening preferences are stored in the local SQLite database.

## Library and control additions

1. Try each library sort, including within a series and a title/author search. Series sorting should use numeric volume order. Missing runtimes should stay last in both length sorts. Restart and check that the selected sort remains.
2. Continue listening starts tracking now; old saved positions do not qualify automatically. Play an ordinary book for five actual minutes, pause and return to the library. It should enter the shelf. A short title qualifies after 5% of runtime, with a 30-second minimum. Brief tests, seeks, buffering and cloud refreshes should not add books. History saves in roughly 10-second batches and when pausing.
3. Remove a shelf book, check it stays removed while listening continues, and try Undo. Explicitly choosing Listen in the library should restore eligibility. The policy excludes 98%-complete books and entries with no listening for 30 days; automated tests cover these time boundaries without altering your history.
4. With focus on a neutral part of the app or mini player, try Space, left/right arrows, B and M. In a search box, note field, selector or focused button, those keys should retain their normal behavior. B creates a local timestamped note labeled Bookmark.
5. With another app focused, try Ctrl+Alt+Space, Ctrl+Alt+left/right, Ctrl+Alt+B and Ctrl+Alt+M. Consult Keyboard & media keys for unavailable combinations. Another app's shortcut registration is not overridden.
6. Try headset/media play/pause and previous/next while both larger windows are hidden. Previous/next should skip 15 seconds within the book. If another app is the active Windows media source, choose this app in Windows media controls and retry. This hardware/OS behavior remains a live test.

All shelf activity and shortcuts operate locally. Windows media metadata contains the current title and author, without account credentials.


## Current book test

1. Set a title as current from its library card. The section above the library should show its cover, progress, Resume and Clear. Nothing should start playing just from setting it.
2. Search or browse a series; Current book should stay visible. Play a different title and return to the app: the chosen current book should remain unchanged.
3. Restart Tauri and check that the pin remains. It does not depend on Continue listening activity, its 30-day cutoff or the 98% completion cutoff. A pinned title should not also occupy Continue listening.
4. Set another title as current, including from the mini player's Settings & help. Check that the library updates. Resume should start that pinned book using the existing mini-player flow.
5. Clear current book. It should stay clear across a restart. History and progress should remain available; the former pinned book can appear in Continue listening if it qualifies.

Current verification: frontend build and 76 frontend tests pass; 44 native tests include replacing/clearing a pin, rejecting invalid or uncached titles and retaining unrelated settings. The updated UI still needs a live test.

## Appearance

Open **Appearance** in the library header, or under the mini player's Settings & help. Choose Navy & gold (the original default), Crimson & champagne, Forest & sage, Ocean & ice, Plum & lavender, Black & green, or White & red. Each palette has coordinated Dark and Light versions, for 14 combinations. Selection is stored locally with listening preferences and shared between the main app and mini player. A local appearance cache avoids reverting to the default while the native preference loads.

Explore now marks the active All titles/series filter with a colored fill, stronger edge, bolder text and a visible marker, plus an accessible pressed state. See series links select the corresponding sidebar item too.

The latest frontend build and all 138 tests pass (93 frontend, 45 native), including palette contrast, Explore selection and loading older preferences with the original default. Browser preview checks confirmed dark/light switching and selected-series styling. Native cross-window theme updates and persistence should also be checked after restarting Tauri.

## Volume test

Restart Tauri. Start a book and try 100%, 50% and 0% in either player. Audio should attenuate and become silent without pausing or changing position. Reset to 100%. Windows system volume and another application's volume should remain unchanged. The slider now stops at 100%, with no boost messages or processing.

Check that both windows show the same saved value, then restart to verify restoration. A previously saved value above 100% should become 100% while retaining speed, theme and other preferences. Tests cover the migration, range validation, silence/reset, mute behavior and media-element telemetry. Live audio levels still need listening verification.

Latest checks: frontend build, 102 frontend tests and 46 native tests.
