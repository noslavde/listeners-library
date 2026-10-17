# Listener’s Library test build

Unofficial Windows x64 companion for Audible US, unaffiliated with Audible or Amazon. This is an unsigned test build. No coding tools are required; the installer can download Microsoft WebView2 if necessary.

1. Close older copies and run the installer. Install over a previous version to retain its encrypted local data.
2. Choose Sign in and complete the official Amazon forms. The library syncs automatically, including on reopening with a remembered account.
3. Choose Listen in mini player. Complete website verification in the official player if Audible requests it.
4. The mini player opens when full-book playback starts. Its return button restores the library while audio continues.
5. Sign out should stop playback, close both players, clear Current Book and hide titles, series and notes. Restart while signed out to verify that cached data stays inaccessible.

Your account tokens and library key use Windows protection. The local library and notes use SQLCipher encryption. Website sign-in uses the local WebView profile. No account credentials or library files are included in the installer. Read PRIVACY.md for protection, retention and removal details.

Report the app version, steps and visible status. Do not share credentials, sign-in URLs, account files, cookies or browser logs. Use the app's bounded Sign-in diagnostic when relevant.

Live testing of account separation, cross-device progress and clean-machine installation remains required before wider release. The app has no automatic updater; install a new setup executable for updates.


## 0.2.0 security update checks

Close older versions before installing. The app is now Listener’s Library and keeps its original Windows data/encryption identifier. Existing credentials remain encrypted and usable; old unassigned notes/history are preserved but not automatically displayed in an account profile. Let the library sync again.

1. Complete a fresh sign-in, confirm library sync, play an owned title, and restart.
2. Save a synthetic note under account A. Sign out during playback. Verify the library, series, notes and mini player disappear and restart stays disconnected.
3. Sign in to account B: A’s notes/history must remain hidden, including when both accounts own the same book. Return to A: A’s newly saved note should return when the provider supplies the same account identity. Legacy credentials or missing identity can create a separate isolated profile.
4. Test website private-browsing mode, expired login, cancellation and playback after verification.
5. If sign-out cleanup reports an error, restart must remain disconnected. New sign-in must retry cleanup first. Do not share real tokens, cookies, account IDs or sign-in URLs with bug reports.

The installer is unsigned. These live tests remain necessary; synthetic tests cannot confirm real provider behavior or every Windows runtime configuration.
