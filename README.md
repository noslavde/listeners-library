# Audible Desktop

A Windows Tauri 2 / React / TypeScript app with a Rust backend and local SQLite library and notes. Audible's official player handles protected audio in a signed-in WebView; the app supplies library browsing, series navigation and an always-on-top mini player. Unaffiliated with Audible.

## Security update 0.1.1

Close old app versions before upgrading. The new installed build encrypts and verifies the old library, then removes its plaintext original. It requires a fresh account sign-in. Review Privacy & local storage to remove old website data, disable remembered website sessions, or erase local data. Development starts a separate empty library. Do not run the old build afterward, since it uses unencrypted storage. Read [protection and recovery limits](docs/PRIVACY.md) before moving or deleting data.

## Windows test installer

Build a shareable installer with `npm run build:windows`. The result is in `src-tauri/target/release/bundle/nsis/`. Testers need Windows 10/11 x64, internet and their own Audible account; they do not need Node, Rust or development tools. The installer can install WebView2 if it is missing. This private test build is unsigned, so Windows may show an unknown-publisher warning. See [release preparation and testing](docs/RELEASE.md).

Release builds disable developer tools and exclude experimental streaming, browser-comparison, manual progress-upload and library-import commands. Only the official player is used for audio. The app identifier stays unchanged, but 0.1.1 uses separate protected stores for development and installed builds. The installed build migrates existing library data; account sign-in is deliberately not imported. No developer account data is packaged.

The collapsed mini player is now 380 × 240 with a 340 × 220 minimum. Chapters, Sleep and More expand on demand; speed, volume, bookmarking, skips, seeking and Back to app remain immediately available.

## Run

Prerequisites: Node 22.12+ (or supported newer Node), Rust stable MSVC, native Windows Perl (for bundled OpenSSL), Visual Studio C++ Build Tools with Windows SDK, and Microsoft Edge WebView2 Runtime.

```powershell
npm ci
npm run dev:desktop  # Windows app with developer tools
npm run dev          # browser demo with simulated playback
npm run build
npm test
```

## Listen

1. Choose **Sign in** and finish Amazon sign-in. The library syncs automatically, including when reopening with a remembered connection.
2. Choose **Listen in mini player** on a book. The app reuses a remembered official launch or finds the title in Audible's library, searching and highlighting it when necessary.
3. Once full-book playback starts, the mini player opens automatically and hides both larger windows. If Audible asks for verification or a manual playback click, complete that in its window first.
4. Use **Back to app ↗** to restore the library while audio continues. Browse series without interrupting playback. **Open mini player** returns to the small player.

The borderless mini player stays on top and has play/pause, 15-second skips, seeking, speed, chapters, cover art when available, chapter/book progress, chapter time remaining and a speed-adjusted finish time. Sleep can stop after 15/30/60 minutes or at the current chapter end, with a countdown, extension and cancel controls. A timer survives closing the mini player but clears on a book change, sign-out or app restart. Speed, rewind amount, pause-after-every-chapter and progress display preferences are saved locally across restarts. Public cover images load from approved Amazon image hosts without a referrer; sync older libraries to obtain artwork. Window placement is saved locally, including during development rebuilds. Closing the whole app stops playback.

The main screen focuses on account connection, books and series. Timestamped notes are under the selected book's Notes section. **Playback help & options** contains official-window recovery, cloud/local resume, pause after chapter, rewind after a long pause, jump recovery and saved progress lookup. These features remain available without occupying the normal library screen.

## Privacy and progress

Credentials are entered only on official Amazon/Audible pages. API tokens stay in the native backend and are stored with Windows user-bound encryption; website sign-in uses a separate WebView2 profile. No account credentials are embedded in code or returned to the frontend. Library metadata, notes, history, settings and playback checkpoints use SQLCipher encryption with a random Windows-protected key. **Sign out** removes this app's remembered API credentials and website cookies while retaining cached books and notes. Development and installed versions now have separate accounts and protected stores. The app remains single-account within each store. See Privacy & local storage for private website sessions and typed-confirmation local data removal.

Official playback supplies its own cloud resume behavior. Read-only cloud-position lookup and local checkpoints are also available. Automatic position uploads are off; cross-device synchronization still needs live verification. See [privacy](docs/PRIVACY.md), [position sync](docs/POSITION_SYNC_TEST.md) and [architecture](docs/ARCHITECTURE.md).

## Current testing

The frontend build, 106 frontend tests and 72 native tests pass. New sleep-timer and preference controls still need live account testing. The user has confirmed the mini-player flow works outside the searched-title failure. Search-result launching now has a fallback for the exact player popup requested by Audible, which still needs a live test. Chapter selection and provider page changes remain areas to test. See the short [next-test checklist](docs/NEXT_TEST.md).

The standalone encrypted-streaming experiment remains in source for reference after Audible rejected its license requests. It has no controls on the main screen, and its browser comparison page is no longer built or packaged. Playback uses the official player, without DRM bypass or content-key extraction.


## Sorting, Continue listening and shortcuts

The library now offers Title A–Z, Author A–Z, Series & book order, Recently listened here, Shortest first, Longest first and Most progress. The chosen sort survives app restarts and also applies within a series or search.

Continue listening shows up to six unfinished titles with verified local listening: at least five minutes, or 5% of a short book with a 30-second minimum. It excludes books at 98% or more and those without listening in the last 30 days. Seeking, paused checkpoints and cloud-position refreshes do not establish listening history. This starts tracking with this update; old position data is not treated as proof of listening. Remove hides a book until it is explicitly opened again; Undo restores it immediately. History remains in the local SQLite database.

Keyboard controls are listed under Keyboard & media keys in the library's Playback help & options and the mini player's More panel. With the app focused: Space toggles play/pause, arrows skip 15 seconds, B saves a timestamped bookmark and M switches the mini player/library. Typing and focused controls retain their normal keys. The same actions are available system-wide with Ctrl+Alt plus those keys. Windows owns registration; unavailable combinations are reported rather than replacing another app's binding.

The app registers a Windows media session using the [desktop media-controls API](https://learn.microsoft.com/en-us/windows/win32/api/systemmediatransportcontrolsinterop/nf-systemmediatransportcontrolsinterop-isystemmediatransportcontrolsinterop-getforwindow). Play/pause controls playback; previous/next rewind/advance 15 seconds within the title. Book title and author are supplied to the local Windows media UI. Hardware behavior and competition with other active media sessions need live testing.


## Current book

Choose **Set as current** on a library card, a Continue listening card, or the playing title under the mini player's More panel. This explicitly pins one main read in the Current book section above the library. It does not start playback, change the playing title, or alter listening history. Playing another book does not replace it.

The selection is stored locally and survives restarts, inactivity and completion. **Resume in mini player** uses the normal official playback flow; **Clear current book** removes the pin. The pinned title is omitted from Continue listening to avoid showing it twice, but its activity is retained. If a sync no longer includes the selected title, the section offers a library sync or a clear action rather than starting an unknown title.


## Themes

**Appearance** in the library header offers seven palettes, each in dark and light: Navy & gold, Crimson & champagne, Forest & sage, Ocean & ice, Plum & lavender, Black & green, and White & red. The same picker is in the mini player's More panel. Your selection is saved locally and shared by both windows. The existing navy/gold dark theme remains the default. Explore highlights the currently selected All titles or series filter, including selections made from See series links.


## App volume

Both players have an app-only 0–100% volume slider, defaulting to 100%, with a reset button. The setting is saved locally; older values above 100% are capped at 100% without resetting other preferences. This changes only the official media element's volume. Windows system volume and other applications are unaffected. There is no boost or extra audio-processing path.

## Sign-in handoff fix 0.1.2

Connection status now survives missed frontend notifications. The library refreshes account state when focused and polls locally while sign-in is finishing. Token-registration and encrypted-save failures remain visible beside account controls. Browser closure after a validated callback no longer cancels an already approved exchange. Callback checks also run after page load and accept fragment-encoded codes only at the same exact trusted HTTPS callback. Existing encrypted stores and keys are unchanged. Live account confirmation is still required.

## Authentication baseline 0.1.3

New connections use the original iOS metadata registration profile recovered from the pre-packaging source snapshot, rather than the Android default introduced for direct-streaming experiments. Existing stored accounts retain their own profile. The original integer-only token-lifetime parser now accepts finite numeric and string decimal values, with bounds and conservative rounding. Registration exchange, encrypted persistence and public connection status are tested together with a fresh synthetic account and loopback provider fixture. This is not yet a live Amazon sign-in confirmation. See [baseline and security reintegration](docs/AUTH_REBASE.md).

## Sign-in diagnostic 0.1.4

Amazon form completion and account connection are separate steps. The app now restores focus to the library after the account exchange, keeps failures visible, and provides a Sign-in diagnostic disclosure beside Account. It identifies the native version/build, connection stage, HTTP status and presence of expected response fields, without returning credentials, account identifiers, provider response text or sign-in URLs. Diagnostic stages stay in memory and clear on a new attempt or restart. A protected account must also pass a read-back check before the app reports connected. Startup account restore errors are now visible instead of silently showing a disconnected account.

The live sign-in failure is still under investigation. Synthetic account exchange, encrypted storage and diagnostic privacy tests pass; they do not verify the real Amazon flow.

## Historical default-browser experiment 0.1.5 (removed in 0.1.6)

Choose **Sign in with browser** to open the Amazon PKCE authorization in the Windows default browser. After completing sign-in, copy the full final Amazon return address into the masked **Amazon return address** field in the app and choose **Connect account**. The provider return page may be blank or show an error; the app validates the final HTTPS origin, path and single authorization code. Do not paste this address into chat. Cancel/restart if the attempt is older than ten minutes. The app never reads the browser cookie store or imports its profile.

Each attempt keeps a fresh verifier in native memory, accepts one result, and uses the existing fixed registration destination and protected storage. Invalid addresses leave the current attempt available; cancellation, expiry, repeated submissions and old generations are rejected. The masked field clears before submission. Passwords and MFA remain in the browser. Browser history/clipboard retention is controlled by the browser and Windows. No arbitrary local redirect server or custom-protocol association is installed.

This connects API library access. Browser cookies are separate from the embedded official player, which may still need its own sign-in. Changing the app’s remember-website setting does not clear default-browser cookies. Live provider sign-in and playback still need verification; the synthetic flow now exercises the browser-return handoff through registration, protected save and reload.

External-browser reference: https://audible.readthedocs.io/en/stable/auth/authorization.html

## First installer account recovery 0.1.6

The user confirmed that 0.1.0 opened already connected; fresh sign-in in that build has not been established. The actual first installer checkpoint is `e21c0ea492beea3cef2d58cc85ec378d11f456dc`, with NSIS bundling enabled. Its fresh-sign-in path used the Android registration profile and shared official-player WebView profile. The previous 0.1.3 rollback incorrectly used an earlier iOS prototype checkpoint.

This build restores the first installer’s Android WebView sign-in selection, removes the external-browser return-address experiment, and retains the current encrypted store, directory protection, release/development separation and secure-save verification. Callback validation and close-race fixes remain. Current library/player features and private website preferences are retained; this is an authentication rollback, not a destructive whole-project reset.

For an account already linked in the old build, open **Account → Restore account from first build**, then **Sync library**. Recovery is explicit and installed-build-only. It reads the original Windows Credential Manager entry inside the native app, preserves the recorded API profile and refresh token, writes Windows-protected storage and checks decryption before reporting connected. No account tokens are sent to the UI, stored as plaintext or included in the installer. Expired access tokens are refreshed through the existing API flow on sync. The original protected credential is retained until the user requests legacy-data removal. It is never imported automatically at startup. If no old credential remains, the app explains that and offers normal sign-in.

See [rollback details](docs/AUTH_REBASE.md). Synthetic tests pass; actual recovery, account refresh and fresh provider sign-in require a live test.

## Registration rejection visibility 0.1.7

The live 0.1.6 diagnostic confirms an HTTP 400 at Amazon account registration, before token parsing or encrypted saving. The app previously discarded error bodies immediately. Registration now inspects non-success bodies only in native memory under a 64 KiB cap and reports allowlisted error codes, fixed message categories and known field-name mentions. Unknown messages/codes and raw bodies are never returned or written to diagnostics. Non-JSON, interrupted and oversized responses retain HTTP status without revealing content. The byte buffer is zeroized. This change does not alter authorization parameters, retry registration, or change encrypted storage. It is a diagnostic build, not a verified fresh-sign-in fix.

## Duplicate registration name fix 0.1.8

The live 0.1.7 result identifies Amazon’s rejection as `DuplicateDeviceName`. Every registration previously sent the same literal `Audible Desktop Lab`. New registrations now use `Audible Desktop` plus a 128-bit hexadecimal suffix derived from the attempt’s existing random device serial. The name is stable within an attempt and changes with a new serial; it contains no username, hostname or account information. Both profiles use this fix. The serial, PKCE, API profile, registration token request and encrypted storage stay unchanged. No devices are deleted or deregistered, and the app does not retry a consumed authorization code. Install the update and start a fresh Sign in.

Tests cover distinct names across synthetic serials, stable repeated construction for one attempt and unchanged original registration fields other than the name. The reported cause is confirmed by the live server error; successful fresh sign-in after the fix still needs a live test.

## Automatic library sync 0.1.9

A connected account now automatically syncs its library after sign-in or explicit account restore and when opening with a remembered account. Each connection starts one refresh; repeated focus/status events do not queue duplicate requests. The button shows Syncing while work runs. Failures retain cached books, show a persistent retry message and leave manual Sync library available. Stale results after disconnect/unmount are ignored. Startup cache loading cannot overwrite a newer sync result. Account registration and encryption remain unchanged.

## Sign-out access boundary 0.1.10

Sign-out now revokes the current account generation before waiting for API requests or browser-cookie cleanup, clears the player hub/current-book selection and visible library/series/notes/player state, cancels playback handoff/sleep, pauses media, closes the mini player and destroys the official player after cookie cleanup. Media keys and player/library entry points require an active account. Cached library, notes, current selection, history and saved-position readers return empty results while signed out; encrypted saved notes and other retained caches are not deleted. On restart without account tokens, those caches stay hidden. Returning connection state invalidates stale cached loads and in-flight UI sync results. API library sync also checks its generation before committing.

Regression coverage includes immediate access revocation while tokens are still in memory and late registration completion staying inaccessible after logout, alongside existing frontend stale-sync tests. Live playback teardown, website cookie removal and signed-out restart still need installed-app verification. This change addresses the disconnected state; it does not introduce separate encrypted database partitions for multiple Audible accounts.

## GitHub builds

The private source repository includes a Windows installer workflow. Main-branch pushes and pull requests run tests and upload a test installer artifact; matching version tags create a draft prerelease for review. See [GitHub build and release instructions](docs/GITHUB_RELEASES.md) and the [tester guide](docs/TESTER_GUIDE.md).
