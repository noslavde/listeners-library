# Windows private test release

Version 0.1.1 is an unsigned, unofficial early test build for Windows 10/11 x64 and the US Audible marketplace. It requires an Audible account and internet. It uses Audible's signed-in official player for audio; it does not decrypt audio itself.

## Build and share

1. Run `npm ci` with the documented Windows development prerequisites.
2. Run `npm test` and `npm run test:native`.
3. Run `npm run build:windows`. This builds the frontend, generates dependency license notices, compiles the optimized native executable and creates a per-user NSIS installer.
4. Share only the resulting installer from `src-tauri/target/release/bundle/nsis/` and the tester instructions. Do not share an AppData folder, WebView profile, credentials, database or a developer checkout.

The installer bundles the app, frontend and third-party license notices. If necessary it downloads Microsoft's WebView2 bootstrapper. Testers do not need development tools. There is no automatic updater; distribute a new installer for each test release.

The application identifier is deliberately preserved for existing users. Version 0.1.1 migrates old library data into encryption, requires fresh sign-in, and separates development/release stores. A new tester starts with their own account. Uninstalling is not a substitute for signing out.

## Release surface

Production has no developer inspection buttons or enabled WebView developer tools. Debug-only command registration excludes the abandoned direct-streaming probe, browser test server, manual progress uploads and library imports. Their research source remains available for development, but these commands are unavailable in a release build. `npm run dev:desktop` explicitly enables tools for development.

Library browsing, account sign-in, official playback, native keyboard/media keys, notes, bookmarks, local checkpoints, read-only cloud progress, themes and listening preferences remain available. The compact player expands for Chapters, Sleep or More and starts collapsed when reopened. Window position remains remembered.

Credentials are not part of the build: account tokens use Windows user-bound DPAPI encryption, website cookies use the user's persistent WebView profile, and library/notes/preferences use their local SQLCipher-encrypted database. Public cover images load from Amazon. No app telemetry or analytics service is included.

## Before a wider public release

This installer has been built successfully, but installation and playback on a second clean Windows machine still need verification. Test installation as a normal user, first sign-in and sync, two different books including one found through library search, resume from mobile, chapter selection, mini-player return, sleep, shortcuts, relaunch and sign-out. Audible page changes and account verification can affect automated opening.

No signing certificate is configured. Windows may identify the publisher as unknown, and some device policies may block unsigned apps. Do not tell testers to disable Windows protections. Obtain a suitable code-signing/reputation distribution path before a general release. Review app branding, distribution rights, dependency notices and an application license before public distribution. The notices generator is attribution assistance, not a complete legal review.

The installer is built with Tauri's standard NSIS tooling; tooling downloads are validated by the Tauri CLI. Record SHA-256 hashes for the exact shared binaries. A successful compilation does not prove clean-machine installation, account compatibility or cross-device progress behavior.

Native builds now require native Windows Perl for the bundled OpenSSL build. Set OPENSSL_SRC_PERL to its executable if it is not on PATH. scripts/native-build.mjs also recognizes the verified portable tool under work/build-tools/strawberry/perl/bin/perl.exe; that temporary tool is not packaged or installed on testers’ machines. Git’s MSYS Perl is not sufficient.
