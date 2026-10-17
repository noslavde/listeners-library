# Experimental macOS validation record

Release candidate: **macos-v0.2.1-alpha.3**, source commit **4c6e844a594c5885376ca9c9a70b244061405c33**.

[Successful build and release-artifact workflow](https://github.com/noslavde/listeners-library/actions/runs/39005448819).

Automated checks passed on both macOS 15 Apple Silicon and Intel GitHub runners:

- 110 frontend tests and 83 native release tests per architecture.
- Native Keychain round-trip, lost-key recovery refusal, authenticated-encryption tamper/wrong-key rejection, SQLCipher data protection, account isolation, private file modes, linked-path rejection and profile locking.
- JavaScript/native dependency audits. The lockfile's existing glib/proc-macro-error informational warnings are outside the selected Mac dependency graph.
- Completed DMGs mounted read-only; image integrity, embedded app signatures and expected CPU architectures verified.
- Platform dependency notices, MIT license, tester guide and SHA-256 checksums included in the draft release.

The Windows native release suite also passed locally (80 tests). Its published 0.2.1 installer and release tag were preserved. The Mac source remains on its own branch and draft PR.

## What this does not verify

No Mac user has tested real Amazon sign-in/MFA, library sync, full-book playback, cross-device resume, WebKit cookie cleanup, mini-player behavior, accessibility or upgrades. macOS 14 is the configured minimum; the native tests ran on macOS 15. There is no independent product security audit.

The apps are ad-hoc signed, not Developer ID signed or notarized. Gatekeeper may block installation. Global shortcuts and system/headset media-key integration are not implemented on Mac.

Keychain-backed encryption is not an app-exclusive or malware-proof vault. Website data lives in a separate WebKit-managed store. Review [Mac privacy and testing instructions](MACOS_EXPERIMENTAL.md) and validate the manual checklist before promoting this alpha.
