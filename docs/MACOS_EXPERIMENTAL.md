# Listener’s Library — experimental Mac alpha

**A separate, untested macOS port of the Windows 0.2.1 beta.** This is not the recommended Windows release. Nobody has verified Amazon sign-in, library sync, full-book audio, the mini player or website sign-out on a real Mac yet. A successful automated build cannot establish that Audible supports this embedded WebKit player.

## Install

Requires **macOS 14 Sonoma or later**, internet access, your own Audible US account and access to the books you play.

Choose **Apple-Silicon** for an M-series Mac or **Intel** for an Intel Mac (Apple menu → About This Mac). Download the **EXPERIMENTAL.dmg**, open it, then drag **Listeners Library Experimental** into Applications. Close other copies before launching. Source ZIP/TAR files are not installers. Draft release downloads are visible only to repository maintainers; workflow artifacts require repository access.

The app is **ad-hoc signed, not signed with a Developer ID and not notarized**. macOS may block it. Do not disable Gatekeeper, remove quarantine attributes or weaken system security to test it. If blocked, report that result and wait for a notarized build. There is no iOS or App Store release.

## Included and unavailable

Library/series browsing, notes, current book, themes, volume, sleep timers and the compact always-on-top mini player use the existing UI. Audio stays inside Audible's official website player. No DRM bypass or audio download is provided.

**Global shortcuts and system/headset media-key integration are unavailable on Mac in this alpha.** In-app shortcuts and buttons remain implemented. Behavior across full-screen apps and Spaces needs testing. Private website mode may require additional sign-in or verification.

## Privacy and recovery

The Mac alpha has its own application identifier, Keychain service and named WebKit website store, separate from Windows, Safari and development builds. It does not import another browser's cookies or a Windows account.

SQLCipher encrypts local library, notes and history. A random macOS Keychain wrapping key protects the database key and saved API account with authenticated AES-256-GCM encryption. There is no plaintext fallback. Missing/damaged keys or denied Keychain access stop recovery and preserve existing files. Do not delete keys to troubleshoot sign-in; there is no developer recovery key.

Keychain does not guarantee app-exclusive access. Protect your macOS login; malware running as you, administrators and backups remain relevant. This experimental build has not had an independent security audit.

Website cookies/cache live separately in WebKit-managed storage; SQLCipher does not encrypt that profile. macOS manages website autofill; the app does not disable it using private APIs. Sign-out stops playback, waits for named-store cleanup completion and checks cookies. This still needs real Mac validation. Erase local app data clears the website store before restart, then removes app files and the wrapping key. Deleting the .app alone does not erase these stores.

Note exports intentionally contain readable, unencrypted text and use owner-only file permissions when created. Keep them private. No account passwords, tokens or keys are packaged or exported. The app adds no analytics/crash uploader; Amazon/Audible pages contact their own services.

## First tester checklist

Report app/macOS version, chip type and visible error text only. Never post account details, sign-in URLs, cookies, raw web console output, local databases or Keychain exports.

1. Open from Applications. Record signature/Gatekeeper warnings. Stop if blocked.
2. Complete Amazon sign-in/MFA; confirm automatic library sync. Quit/reopen and check account/library restoration.
3. Play an owned title using official **Listen now**. Confirm actual sound, moving counters, full-book chapter access and the expected resume position. A sample or frozen player is a failed test.
4. Verify the mini player opens, hides the main windows, stays above ordinary windows, drags/resizes and returns to the library without interrupting audio.
5. Try play/pause, ±15 seconds, speed, volume, chapter selection, sleep timer and in-app shortcuts. Browse a series while listening.
6. Create/edit/delete/export notes; quit/reopen and verify persistence. A second copy must refuse to share the open profile.
7. Sign out during playback. Audio/windows/current book/series must disappear. Restart must stay disconnected; another account must not inherit the first account's books/notes.
8. Turn website remembering off, sign in/play, quit/reopen, and verify private website state is not retained. Record extra verification requests.
9. Export notes you want to keep, then test **Erase local app data**. It must restart disconnected. Report errors; never delete keys by hand.

Cross-device resume remains governed by Audible's official player and is not guaranteed; compare against web/mobile and report discrepancies.

## Build and distribution

The source branch is `codex/macos-experimental`. The separate **macOS experimental** workflow runs frontend tests, native tests including Keychain/encryption, dependency checks and ad-hoc signed DMG builds on Apple Silicon and Intel Mac hosts. It verifies signatures and records SHA-256 checksums. Workflow artifacts are available for 14 days.

The `macos-v0.2.1-alpha.1` tag creates a **draft prerelease**, never automatically published or marked latest. Windows release assets are preserved. Keep the Mac release in draft until a Mac tester completes the checklist and the Windows workflow's Mac-tag exclusion is merged to the default branch. A compile/test pass is not a playback pass.

Build on a Mac with Xcode command-line tools, Node 24 and stable Rust:

    npm ci
    cargo fetch --locked --manifest-path src-tauri/Cargo.toml
    npm run build:macos

Use `npm run dev:macos` for development; it has a separate private store. The Mac app cannot be compiled or run locally on this Windows computer.

Wider distribution needs Developer ID signing, Apple notarization and clean-Mac testing. Configure future signing credentials in protected CI secrets; never commit them or paste them into chat. The current config uses ad-hoc signing and no Apple account credentials.

References: [Tauri signing](https://v2.tauri.app/distribute/sign/macos/), [application bundles](https://v2.tauri.app/distribute/macos-application-bundle/), [named WebView stores](https://docs.rs/tauri/2.12.3/tauri/webview/struct.WebviewWindowBuilder.html#method.data_store_identifier), [GitHub Mac runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
