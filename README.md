# Listener’s Library

A Windows audiobook companion with an always-on-top mini player, your library and series, timestamped notes, sleep timers and keyboard/media controls. Independent and unaffiliated with Audible or Amazon.

**0.2.1 is a beta.** Supports Windows 10/11 x64 and Audible's **US marketplace**. You need your own account, access to the titles you play and an internet connection. No offline listening or audio downloads are provided. The unofficial account integration and website automation may break when Audible changes its services.

## Download and install

1. Open [Releases](https://github.com/noslavde/listeners-library/releases). Choose the latest beta and expand **Assets**.
2. Download **Listeners-Library-…-Windows-x64-Setup.exe**. The **Source code** ZIP/TAR files are for developers, not installation.
3. Close all running copies of the app, then run Setup. No coding tools are required. Setup can download Microsoft's WebView2 runtime if it is missing.
4. The installer is unsigned, so Windows may show an unknown publisher or block it. Do not turn off Windows protections. If your device policy blocks it, stop and ask the maintainer for a supported distribution option.

Private repositories require GitHub access even to download releases. Source is [MIT licensed](LICENSE); third-party components have their own notices.

## Start listening

1. Choose **Sign in** and complete the official Amazon forms, including verification. Your library syncs automatically.
2. Choose **Listen in mini player** on a book. The app opens Audible and finds its **Listen now** button; the first launch can take longer than later launches.
3. If Amazon asks for verification, complete it in that window. If automatic opening stops, use **Listen now** beside the highlighted book.
4. When full-book playback starts, the mini player opens and the larger windows hide. Its **↗** button returns to the library while audio continues.

See [Getting started & troubleshooting](docs/TESTER_GUIDE.md) for notes, progress, shortcuts, updating and removal.

## Your data

Account tokens and local library/notes are protected on this Windows account. Website sign-in uses a separate local browser profile. No app analytics or crash uploader is added; Audible's official pages still contact their own services. This is not an independently audited product. Read [Privacy](docs/PRIVACY.md) and [Security](SECURITY.md).

**Before updating older builds:** close them all. Updating Listener’s Library 0.2.0 retains its account profiles. Older unassigned notes/history are kept encrypted but do not automatically appear in a new account profile. Under **Notes & backups**, you can explicitly export old notes to a private JSON file. No automatic ownership guess or merge is made. Keep the encryption key and Windows profile; there is no developer recovery key.

The earlier Audible Desktop installation may remain as a separate entry. Use only Listener’s Library and remove the old shortcut to avoid opening an unsafe older build. See the guide before deleting local data.

## For developers

Tauri 2, React, TypeScript, Rust and SQLCipher. Audio stays inside Audible's official player; there is no DRM bypass or audio-key extraction.

Windows build requirements: Node 22.12+ (CI uses Node 24), Rust stable MSVC, Visual Studio C++ Build Tools/Windows SDK, native Windows Perl for bundled OpenSSL, and Edge WebView2.

    npm ci
    npm run dev:desktop
    npm test
    npm run test:native -- --release --locked
    npm run build:windows

`npm run dev` is a browser demonstration with simulated playback. It does not sign in or play purchased titles. The real app uses `dev:desktop`. Development and installed builds intentionally use different private stores.

See [architecture](docs/ARCHITECTURE.md), [release checklist](docs/RELEASE.md), [GitHub build/release instructions](docs/GITHUB_RELEASES.md), and [security review](docs/SECURITY_REVIEW.md).
