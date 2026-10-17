# Listener’s Library

An independent Windows audiobook companion with library and series browsing, timestamped notes, and a compact always-on-top player. Built with Tauri 2, React, TypeScript, Rust and SQLCipher. Unaffiliated with Amazon or Audible.

Playback runs through the official Audible website in an isolated WebView2 window. The app adds controls and navigation without extracting audio keys or bypassing DRM. Account/library access uses an unofficial integration and may stop working when the provider changes its services. Currently supports the US marketplace.

## Version 0.2.0: account protection

Account libraries, notes, history, positions and current-book selections use separate encrypted databases. Sign-out stops account access immediately and persists a restore block before cleanup, so a leftover credential cannot silently reconnect after restart. Website cleanup waits for WebView2's full browsing-data deletion callback. The retired first-build account recovery action is no longer exposed.

**Upgrading:** close every older app version first. The original internal app identifier, encryption contexts and credential location remain unchanged. Existing credentials can still unlock. Old unassigned notes/history remain encrypted in the previous database, but are not automatically displayed or assigned to a new account. Let the library sync again. General appearance/player preferences remain available. Read [privacy and recovery limits](docs/PRIVACY.md) before changing or deleting local data.

The renamed installer can coexist with the old product entry. Do not run both, or return to an older build after testing this account boundary. This release does not automatically uninstall the old product or erase its files.

## Listening

1. Choose Sign in and complete Amazon's forms. Your library syncs automatically.
2. Choose Listen in mini player on a book. The app reuses a remembered official launch or locates the title in the signed-in website library, searching and highlighting it when necessary.
3. Complete any verification/playback prompt in the official window. Once full-title playback is ready, the mini player opens and both larger windows hide.
4. Back to app restores the library while playback continues. Browse series without interruption.

The mini player includes play/pause, 15-second skips, seeking, speed, 0–100% app volume, chapters, chapter/book progress, speed-adjusted finish time, sleep timers, jump recovery and rewind after a long pause. The library adds sorting, Current book, Continue listening, timestamped notes and light/dark color themes. Keyboard shortcuts and Windows media controls are available. Window placement and preferences persist locally.

## Privacy

Account tokens stay in the native backend and are saved with Windows current-user DPAPI protection. Library data uses SQLCipher with a random Windows-protected key, without a plaintext fallback. Remote provider pages have no granted Tauri capabilities. Development and installed builds have separate protected stores.

The official website has its own WebView2 browser profile; it is not encrypted by SQLCipher. Private website sessions are available under Privacy & local storage. Sign-out retains encrypted per-account notes/history; Erase local app data removes retained local files after typed confirmation. No app analytics, credential export or remote notes service is added. Provider pages may collect their own telemetry.

Windows-user protection is not an app-exclusive vault: malware running as you or an administrator can still access decrypted data. There is no developer recovery key. See [privacy details](docs/PRIVACY.md).

## Running and building

Requires Windows 10/11 x64, Node 22.12+ or a supported newer Node, Rust stable MSVC, Visual Studio C++ Build Tools/Windows SDK, native Windows Perl for bundled OpenSSL, and Microsoft Edge WebView2.

```sh
npm ci
npm run dev:desktop   # app with developer tools
npm run dev           # browser demo with simulated playback
npm test
npm run test:native -- --locked
npm run build:windows
```

The Windows build generates dependency notices, checks release safeguards and creates an NSIS installer under src-tauri/target/release/bundle/nsis/. The installer is unsigned. Testers need their own account and internet access, but no coding tools. Release builds exclude experimental streaming, browser-comparison, manual upload and developer commands.

## Verification and distribution

106 frontend and 75 native tests pass, including synthetic encrypted-storage, tamper, account-isolation, restart-after-sign-out and registration tests. The production frontend build and release checks pass. Live account switching, website cleanup/private mode, playback and clean-machine installation still require testing; this is not an independent security audit. Rust dependency advisory auditing remains outstanding.

The private [GitHub repository](https://github.com/noslavde/listeners-library) builds test packages on main/PR changes. Matching version tags create draft prereleases, without automatically publishing them. See the [tester guide](docs/TESTER_GUIDE.md), [GitHub build instructions](docs/GITHUB_RELEASES.md), [release checklist](docs/RELEASE.md), and [dependency notices](docs/THIRD_PARTY_NOTICES.md).

No public release, code-signing certificate, automatic updater or application license is configured. Naming is not a trademark clearance or an authorization to use the provider's services. Keep distribution private while release checks and service-terms review remain open.
