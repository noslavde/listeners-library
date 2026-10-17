# Security policy

Listener’s Library is an unofficial Windows beta. It has not received an independent security audit. The latest beta is the supported version; do not use older builds to bypass account separation or recover notes.

## Reporting a concern

Use GitHub's private vulnerability-reporting option if it is available for this repository. Otherwise, open an issue asking the maintainer for a private reporting channel without describing an exploitable issue publicly. Never attach passwords, tokens, cookies, account files, encryption keys, signed-in URLs or full browser logs. A synthetic example and the app version are usually sufficient to start.

## Boundaries

- Library, notes and history are encrypted with SQLCipher. The random database key and account tokens use Windows current-user protection. The key is not bundled with the app.
- This protects against ordinary cross-user access and accidental copying, not malware running as the same Windows user, administrators or a compromised operating system. There is no developer recovery key.
- Official website storage is managed by WebView2, separately from the encrypted library. Website password saving/autofill is disabled; optional private browsing reduces normal website persistence.
- Remote provider windows have no local Tauri capabilities. Local native commands validate the window and local origin; the main window alone may access notes, account or library operations.
- Sign-out revokes access before cleanup, persists a restart block and reports cleanup failures. Retry incomplete sign-out before sharing the Windows session.
- Note exports require explicit confirmation and an OS save dialog. Exported JSON is readable and is no longer protected by the app's encryption. Old unassigned notes are never automatically merged into an account.
- The installer is unsigned. Do not disable Windows security to install it. No automatic updater is included.

## Release checks

CI runs JavaScript and native tests, production build safeguards, npm advisory checks and cargo-audit against locked dependencies. Known vulnerability findings fail the audit; informational upstream notices are reviewed separately. Passing these checks is not proof of an absence of vulnerabilities. Live account switching, interrupted cleanup, private website mode and fresh-machine installation must also be checked for a release.

See [the dated security review](docs/SECURITY_REVIEW.md) for evidence and remaining limitations, and [privacy](docs/PRIVACY.md) for retention and removal.
