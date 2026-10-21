# Listener’s Library v0.2.7 beta — Windows

Download **Listeners-Library-0.2.7-Windows-x64-Setup.exe** under **Assets** to install. Source-code ZIP/TAR archives are for developers. Close every running copy before updating; save unsaved drafts first. Windows 10/11 x64, WebView2, internet and your own Audible US account are required.

This release collects the Windows improvements since v0.2.1. The installer remains unsigned; do not disable Windows protections if your device policy blocks it. macOS work remains experimental and is not included in this Windows release.

## Library and player

- Main window sections: **Library**, **Bookmarks & notes**, **My reviews**, and **Settings**. Account controls, privacy and diagnostics are under Settings.
- Main and mini windows share player controls: play/pause, ±15 seconds, seeking, speed, app volume, chapter navigation, sleep, history, saved moments and explicit cloud-position sync.
- A more visible current chapter, restart-then-previous navigation, larger cover-artwork popup and correctly aligned volume labels.
- Explicit **Sync position** for returning from mobile. Dated PC checkpoints and bounded history help recover progress; conflicting PC/Audible positions require a choice because the newest mobile listening time is not verified.
- Improved title switching, main-window close/reopen behavior, single-instance activation and recovery from hidden windows.

## Saved moments and private reviews

- Search/filter/edit bookmarks, timestamped notes and clip ranges. Capture a bookmark or the last 30 seconds from either player. Clips replay through the official player; no audio files are captured.
- Local ratings from **0.0–5.0** in 0.1 increments, optional reviews, card scores and remembered rating-based sorting. Zero and unrated are distinct. Reviews stay encrypted for this account on this PC and never sync to Audible.
- Optional **experimental annotation sync**: explicitly read mobile items, send individual eligible items/text edits and review conflicts. No automatic background sync or cloud deletions. Uncertain sends block blind retry, and conflicts retain local copies.

## Security and maintenance

- Updated rustls-webpki to 0.103.16 for certificate-validation advisory RUSTSEC-2026-0382. The account/login exchange and official DRM playback route are preserved.
- Bounded diagnostic reports contain allowlisted categories and timing, with user-reviewed local export and no automatic upload.
- Repository-wide readable formatting, feature organization, module summaries, contract comments, a developer walkthrough and source index.
- Existing protected account and encrypted data stores are retained. Sign-out hides/revokes account access; it does not automatically erase retained encrypted profiles.

## Beta limits and testing

Automated frontend/native tests, formatting, production/release checks and dependency audits are run for this build. No independent security audit has been performed. Current live upgrade, account-switching and mobile round-trip coverage is limited; automated tests do not prove those scenarios.

**Annotation sync is not verified against live mobile accounts.** Keep important notes exported and start with disposable items using [ANNOTATION_SYNC_TEST.md](ANNOTATION_SYNC_TEST.md). Service changes can break unofficial integration. Protected standalone playback, offline audio downloads, cloud review/rating sync, code signing and an automatic updater are not provided.

Start with **TESTER_GUIDE.md** and **NAVIGATION_REVIEWS_TEST.md** under Assets. Playback of clips changes the official player's real timeline and may sync that position; use Return to listening when appropriate. Use Settings → Troubleshooting & diagnostic report to review a report before sharing. Never share passwords, account files, cookies, signed-in URLs, databases or private notes/reviews.
