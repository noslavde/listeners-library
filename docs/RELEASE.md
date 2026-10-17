# Maintainer release checklist — 0.2.1 beta

This is a beta, not an independently audited stable release. User installation and troubleshooting instructions are in TESTER_GUIDE.md. Development history is not a user guide.

## Automated gates

1. npm ci; npm audit --audit-level=low; npm test.
2. cargo fetch --locked --manifest-path src-tauri/Cargo.toml.
3. cargo audit --file src-tauri/Cargo.lock (review informational notices too).
4. npm run build; npm run notices; npm run check:release.
5. npm run test:native -- --release --locked.
6. npm run build:windows; record SHA-256 of the exact setup executable.
7. Review packaging: installer, TESTER_GUIDE.md, PRIVACY.md, THIRD_PARTY_LICENSES.txt, LICENSE and SHA256SUMS.txt. Never upload profiles, account files, keys or databases.

CI runs these dependency/test/build gates and creates a draft prerelease on a matching version tag. See GITHUB_RELEASES.md. Push main to validate before tagging; do not overwrite an existing released binary or move an old tag.

## Live gates — record passed/failed, do not infer from unit tests

- Clean Windows 10/11 x64 normal-user installation, with WebView2 present and missing.
- Fresh Amazon sign-in with verification, cancel, restart, expired credentials and automatic library sync.
- Play two owned books, including one requiring library search; show official player when automatic launch fails. Confirm audio, seeking, speed and chapters.
- Mobile/browser resume: compare actual chapter/time; verify conflict display and explicit local/cloud resume choices.
- Book A draft → book B → book A; ensure draft/timestamp stay with A. Save/edit/delete notes, read without playback, cancel/export JSON to a private folder and inspect synthetic output.
- Upgrade from 0.2.0: existing notes remain. Older shared store: explicit old-note export preserves originals and never auto-merges accounts. Missing-key error preserves files.
- Sign out during playback, library sync and note export; restart disconnected. Test A/B isolation, including shared titles, and return to A. Test cleanup failure/retry and website private mode.
- Mini player/return/close, keyboard-only operation, media keys, sleep, 125–200% display scaling, multiple monitors and screen reader labels.
- Export before erase; cancel destructive confirmations; confirm uninstall does not imply sign-out.

## Distribution limits

The installer is unsigned, with no automatic updater. Do not tell testers to disable Windows security. Code signing requires a suitable publisher identity/certificate or distribution service and is not solved by passing tests. Do not market this beta as security-verified. The code is MIT licensed; dependency notices and provider service/branding permissions are separate matters. Source/release visibility follows repository settings.

Keep unverified gates visible in release notes. Testers need their own account and internet, not developer tools. The older Audible Desktop product can remain separately listed; do not run it alongside or instead of the current build.
