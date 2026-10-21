# Listener’s Library

A Windows audiobook companion with an always-on-top mini player, your library and series, timestamped notes, sleep timers and keyboard/media controls. Independent and unaffiliated with Audible or Amazon.

**0.2.7 is a Windows beta.** Supports Windows 10/11 x64 and Audible's **US marketplace**. You need your own account, access to the titles you play and an internet connection. No offline listening or audio downloads are provided. The unofficial account integration and website automation may break when Audible changes its services.

## Download and install

1. Open [Releases](https://github.com/noslavde/listeners-library/releases). Choose the latest beta and expand **Assets**.
2. Download **Listeners-Library-…-Windows-x64-Setup.exe**. The **Source code** ZIP/TAR files are for developers, not installation.
3. Close all running copies of the app, then run Setup. No coding tools are required. Setup can download Microsoft's WebView2 runtime if it is missing.
4. The installer is unsigned, so Windows may show an unknown publisher or block it. Do not turn off Windows protections. If your device policy blocks it, stop and ask the maintainer for a supported distribution option.

Private repositories require GitHub access even to download releases. Source is [MIT licensed](LICENSE); third-party components have their own notices.

## Start listening

1. Open **Settings → Sign in** and complete the official Amazon forms, including verification. Your library syncs automatically.
2. Choose **Listen in mini player** on a book. The app opens Audible and finds its **Listen now** button; the first launch can take longer than later launches.
3. If Amazon asks for verification, complete it in that window. If automatic opening stops, use **Listen now** beside the highlighted book.
4. When full-book playback starts, the mini player opens and the larger windows hide. Its **↗** button returns to the library while audio continues.

See [Getting started & troubleshooting](docs/TESTER_GUIDE.md) for notes, progress, shortcuts, updating and removal.

Closing the main library exits the app; closing the mini player returns to the library. There is no tray mode. Launching the shortcut again restores the existing library. Under **Settings**, **Troubleshooting & diagnostic report** offers a small report to review and save yourself, with no automatic uploads or account/book content. See [close/reopen tests](docs/LIFECYCLE_TEST.md).

## Your data

The main window has Library, Bookmarks & notes, My reviews and Settings sections, with the same player controls in both windows. Private local ratings (0.0–5.0) and reviews appear on book cards and support sorting by your rating. See [the navigation and reviews checklist](docs/NAVIGATION_REVIEWS_TEST.md).

Optional, explicit Audible annotation sync can read mobile annotations, send individual eligible items and text edits, and preserve conflicts for review. It does not perform automatic sync or cloud deletions. Follow [the mobile round-trip test](docs/ANNOTATION_SYNC_TEST.md) before trusting it with important notes; live mobile compatibility is not yet verified.

**Bookmarks, clips & notes** offers searchable saved moments, editable clip ranges and mini-player capture. Clips replay through the official player; they are metadata, not audio files. They stay local unless you explicitly use the experimental annotation-sync controls for eligible items. See [saved-items checks](docs/BOOKMARKS_TEST.md).

Account tokens and local library/notes are protected on this Windows account. Website sign-in uses a separate local browser profile. No app analytics or crash uploader is added; Audible's official pages still contact their own services. This is not an independently audited product. Read [Privacy](docs/PRIVACY.md) and [Security](SECURITY.md).

**Before updating older builds:** close them all. Updating Listener’s Library 0.2.0 retains its account profiles. Older unassigned notes/history are kept encrypted but do not automatically appear in a new account profile. Under **Bookmarks, clips & notes → Export & older notes**, you can explicitly export old notes to a private JSON file. No automatic ownership guess or merge is made. Keep the encryption key and Windows profile; there is no developer recovery key.

The earlier Audible Desktop installation may remain as a separate entry. Use only Listener’s Library and remove the old shortcut to avoid opening an unsafe older build. See the guide before deleting local data.

## For developers

See [contributing and code organization](CONTRIBUTING.md) for the source map,
readability conventions, and repository-wide formatting commands.

Tauri 2, React, TypeScript, Rust and SQLCipher. Audio stays inside Audible's official player; there is no DRM bypass or audio-key extraction.

Windows build requirements: Node 22.12+ (CI uses Node 24), Rust stable MSVC, Visual Studio C++ Build Tools/Windows SDK, native Windows Perl for bundled OpenSSL, and Edge WebView2.

    npm ci
    npm run dev:desktop
    npm test
    npm run test:native -- --release --locked
    npm run build:windows

`npm run dev` is a browser demonstration with simulated playback. It does not sign in or play purchased titles. The real app uses `dev:desktop`. Development and installed builds intentionally use different private stores.

See [architecture](docs/ARCHITECTURE.md), [release checklist](docs/RELEASE.md), [GitHub build/release instructions](docs/GITHUB_RELEASES.md), and [security review](docs/SECURITY_REVIEW.md).
