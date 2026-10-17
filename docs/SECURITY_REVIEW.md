# Security review — October 16, 2026

Scope: local source and release checks for Listener’s Library 0.2.1 beta. This is a maintainer review, not an independent audit or a guarantee that the app is vulnerability-free. Tests use synthetic data; no real Amazon tokens, cookies or account stores were inspected for this work.

## Dependency findings

- npm audit: zero known vulnerabilities across production and development dependencies at review time.
- cargo-audit 0.22.2 initially identified four RustSec advisories against locked rustls 0.23.45. The compatible lockfile update to 0.23.46 resolves all four; repeat audit reports zero known vulnerability findings.
- Informational notices remain: proc-macro-error 1.0.4 is unmaintained (RUSTSEC-2024-0370), and glib 0.18.5 has an unsound iterator (RUSTSEC-2024-0429). Neither appears in `cargo tree --target x86_64-pc-windows-msvc`; they are retained by upstream dependencies for other platforms. This release supports Windows x64 only. These notices have not been suppressed; do not claim other platforms are audited.
- Windows network requests use the platform TLS path. rustls was not present in the filtered Windows dependency tree, but its locked version was patched rather than leaving a known-vulnerable package in the all-platform lockfile.
- CI now audits npm and Rust dependencies and fails on known vulnerability findings. Advisory results can change after this date; repeat them for each release.

Primary sources: [RustSec advisory database](https://rustsec.org/advisories/), [rustls security policy](https://github.com/rustls/rustls/security/policy), [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit).

## Changes reviewed

- Preserved the working Android account registration protocol, Windows protection contexts and original data identifier. No encryption downgrade, secret export or automatic account-ownership guess was introduced.
- Central local command guards now check local origin as well as the window label; lookalike/provider origins are rejected. Remote capability lists remain empty.
- Note access captures the account database under a lifecycle guard. Export rechecks the originating account after the save dialog, so changing accounts cancels it before writing. Export includes notes/book identifiers only, through a native file picker and explicit typed confirmation. JSON is intentionally readable and its privacy consequences are explained.
- Old shared notes remain encrypted; explicit old-note export is available without importing/merging into the signed-in profile. Registration-derived profiles still cannot guarantee continuity after a fresh sign-in; the app warns when provider identity is absent.
- Sign-in can be canceled; incomplete sign-out exposes Retry sign-out; sign-out is available during library sync. The durable sign-out block is retained.
- Book-specific drafts prevent text/timestamp moving to another book. Conflicting saved positions no longer imply that a recent cloud check is recent listening. User docs and error recovery now describe these limits.

## Remaining release gates

The installer is unsigned. No code-signing identity, automatic updater or independent security assessment was added. A second clean PC and live accounts are still needed to verify MFA/expired login, account switching, private website behavior, interrupted cleanup, sign-out while export is open, real audio/chapter selection, mobile progress and accessibility/display scaling. Follow RELEASE.md; do not mark these passed from unit-test results.

Recovery is intentionally limited: old shared notes can be exported as readable JSON, but unknown ownership is not auto-assigned, histories are not migrated, there is no note import, and lost Windows-protected keys cannot be recovered by the developer. Keep this a beta until those limits and the live checks are acceptable for the intended audience.
## Local verification results

The final frontend suite passes 110 tests. The native optimized Windows suite passes 80 tests, including encryption/tamper/ownership tests, local-origin rejection, note validation/edit boundaries and complete export replacement. The frontend production build, packaging safeguards, formatting checks, workflow syntax and mocked release-attachment paths pass. The Windows NSIS installer is built locally; it is unsigned.

A synthetic browser UI check verified Book A draft → Book B draft → return to A, retained initial timestamp, saving, editing and disabled export without confirmation. This uses mocked local commands and does not replace testing native file dialogs, account cleanup or actual Audible playback. Fresh-machine installation and the live gates above remain unverified.
