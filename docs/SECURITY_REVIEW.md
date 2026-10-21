# Security review and release checks

## 0.2.7 Windows beta — October 20, 2026

This release includes player/history/lifecycle improvements, saved moments, optional experimental annotation sync, local reviews and a source-readability/documentation pass. These are maintainer checks, not an independent audit or a promise that account/service integration cannot fail.

- Fresh npm audit: no known vulnerability findings.
- Fresh cargo-audit found RUSTSEC-2026-0382 in rustls-webpki 0.103.15. The lockfile now pins the fixed 0.103.16 patch. A refreshed audit reports no vulnerability findings and the two previously documented informational warnings below; none were suppressed.
- Frontend/native regression suites, production bundling, formatting, release resource assertions and release-workflow tests cover the release source. GitHub Actions repeats the automated gates before creating the release draft.
- Account/sign-in and official-player DRM routes are retained. No private stores, account keys, real credentials or local browser profiles belong in the source or release assets.
- Experimental annotation sync remains opt-in, explicit and unverified against live mobile accounts. Preflight reads are not atomic server-side conditional writes; do not promise conflict-free simultaneous phone/PC editing.
- Current Windows upgrade/account-switching and live mobile round-trip coverage is limited. Follow the attached test guides with disposable saved items. A passing unit test or dependency audit is not evidence of real account isolation, an actual audio session, or the newest cloud listening position.
- The installer is unsigned and no automatic updater or diagnostic uploader is enabled.

## Historical 0.2.1 maintainer review — October 16, 2026

Scope: local source and release checks for Listener’s Library 0.2.1 beta. This is a maintainer review, not an independent audit or a guarantee that the app is vulnerability-free. Tests use synthetic data; no real Amazon tokens, cookies or account stores were inspected for this work.

### Dependency findings

- npm audit: zero known vulnerabilities across production and development dependencies at review time.
- cargo-audit 0.22.2 initially identified four RustSec advisories against locked rustls 0.23.45. The compatible lockfile update to 0.23.46 resolves all four; repeat audit reports zero known vulnerability findings.
- Informational notices remain: proc-macro-error 1.0.4 is unmaintained (RUSTSEC-2024-0370), and glib 0.18.5 has an unsound iterator (RUSTSEC-2024-0429). Neither appears in `cargo tree --target x86_64-pc-windows-msvc`; they are retained by upstream dependencies for other platforms. This release supports Windows x64 only. These notices have not been suppressed; do not claim other platforms are audited.
- Windows network requests use the platform TLS path. rustls was not present in the filtered Windows dependency tree, but its locked version was patched rather than leaving a known-vulnerable package in the all-platform lockfile.
- CI now audits npm and Rust dependencies and fails on known vulnerability findings. Advisory results can change after this date; repeat them for each release.

Primary sources: [RustSec advisory database](https://rustsec.org/advisories/), [rustls security policy](https://github.com/rustls/rustls/security/policy), [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit).

### Changes reviewed

- Preserved the working Android account registration protocol, Windows protection contexts and original data identifier. No encryption downgrade, secret export or automatic account-ownership guess was introduced.
- Central local command guards now check local origin as well as the window label; lookalike/provider origins are rejected. Remote capability lists remain empty.
- Note access captures the account database under a lifecycle guard. Export rechecks the originating account after the save dialog, so changing accounts cancels it before writing. Export includes notes/book identifiers only, through a native file picker and explicit typed confirmation. JSON is intentionally readable and its privacy consequences are explained.
- Old shared notes remain encrypted; explicit old-note export is available without importing/merging into the signed-in profile. Registration-derived profiles still cannot guarantee continuity after a fresh sign-in; the app warns when provider identity is absent.
- Sign-in can be canceled; incomplete sign-out exposes Retry sign-out; sign-out is available during library sync. The durable sign-out block is retained.
- Book-specific drafts prevent text/timestamp moving to another book. Conflicting saved positions no longer imply that a recent cloud check is recent listening. User docs and error recovery now describe these limits.

### Remaining release gates

The installer is unsigned. No code-signing identity, automatic updater or independent security assessment was added. A second clean PC and live accounts are still needed to verify MFA/expired login, account switching, private website behavior, interrupted cleanup, sign-out while export is open, real audio/chapter selection, mobile progress and accessibility/display scaling. Follow RELEASE.md; do not mark these passed from unit-test results.

Recovery is intentionally limited: old shared notes can be exported as readable JSON, but unknown ownership is not auto-assigned, histories are not migrated, there is no note import, and lost Windows-protected keys cannot be recovered by the developer. Keep this a beta until those limits and the live checks are acceptable for the intended audience.

### Local verification results

The final frontend suite passes 110 tests. The native optimized Windows suite passes 80 tests, including encryption/tamper/ownership tests, local-origin rejection, note validation/edit boundaries and complete export replacement. The frontend production build, packaging safeguards, formatting checks, workflow syntax and mocked release-attachment paths pass. The Windows NSIS installer is built locally; it is unsigned.

A synthetic browser UI check verified Book A draft → Book B draft → return to A, retained initial timestamp, saving, editing and disabled export without confirmation. This uses mocked local commands and does not replace testing native file dialogs, account cleanup or actual Audible playback. Fresh-machine installation and the live gates above remain unverified.

### Owner smoke test — October 16, 2026

The owner reported that sign-in, library loading and owned-book playback in the mini player work with the local 0.2.1 installer on their existing PC. This is a user-reported pass for those three checks. It does not establish fresh-machine installation, MFA/expired-session coverage, cross-account isolation, sign-out cleanup, mobile resume or all chapter controls; those remain separate release gates.

### Windows lifecycle and diagnostics follow-up — October 18, 2026

Local 0.2.4 test build: main-window close explicitly exits after saving confirmed progress; hidden playback views cannot intentionally keep the process alive. A per-user, per-scope Windows mutex/event pair activates an existing library on another launch. The storage file lock remains in place. Startup restoration, mini destruction recovery and a two-second check for a hidden library without a visible mini player address visibility failures. Normal minimization is retained. Setup failures display a fixed message rather than raw error details.

Diagnostics use a strict event enum and numeric relative times, not message-string redaction. There is no free-form error, URL, path, account, book, note, stack or memory-dump field. Persisted input rejects unknown fields, invalid codes, oversized files and excess events. The plaintext journal lives in a reparse-point-checked directory restricted to the Windows owner and SYSTEM. It keeps two sessions of at most 128 events each. Account stores and listening history are never included. Local-only command guards reject remote provider pages, and frontend failures forward only a fixed category. Panic recording uses a nonblocking lock to avoid a recursive logging deadlock. Logging is best effort and intentionally cannot diagnose all hard native crashes.

Exports require a reviewed snapshot matching the native report and use a native save dialog, bounded JSON and atomic replacement. Cancellation creates no export. There is no upload endpoint or stable tracking identifier. Same-user malware, administrator access, user-supplied report context and independently cloud-synced export destinations remain outside these protections. Erase removes prior current-scope diagnostic history; a new session starts after restart.

Verification: 120 frontend tests and 96 optimized native tests pass, including Windows activation/restart handoff, bounded diagnostic parsing/classification and exclusion of synthetic secrets from error forwarding. Production frontend and formatting checks pass. A synthetic browser check verifies report review and cancellation UI. No real account files were inspected. Native window activation, actual close/reopen, installer upgrade/uninstall and real account playback still require the owner checks in LIFECYCLE_TEST.md; this is not an independent security audit.

### Saved items follow-up — October 20, 2026

Local 0.2.5 adds encrypted clip-range metadata beside existing note rows; it does not capture audio, add DRM access or contact a new service. An additive details table and deletion trigger preserve legacy note IDs and prevent stale range details from attaching to reused IDs. Editing/deleting compare the complete previous item within a database savepoint. Queued saves, bookmarks, replay and confirmed position choices retain the originating account generation. Native replay reads the stored range, checks the active full-title snapshot and current Audible WebView origin, and validates the real seekable range before queuing playback. Remote provider pages still have no access to local commands. No new dependencies were added.

Frontend validation and native validation reject invalid times, out-of-book offsets, oversized text and clip ranges over five minutes. The bridge respects the initial progress-review hold, cancels the end boundary on manual seek/chapter controls, and pauses at the end using media time updates plus the normal polling fallback. Replay changes the real official timeline; a return action confirms and records the original position explicitly. It is not a separate silent preview and cannot prevent Audible from observing intermediate positions. Saved clip/note content remains absent from diagnostic reports; explicit readable exports include ranges and text.

Verification: 126 frontend and 99 optimized native tests pass. Coverage includes clip end pausing, manual cancellation, held/unsupported replay, UTF-8 text limits, timestamp parsing, old-row preservation, stale edits, invalid ranges and deletion/ID reuse. Synthetic browser checks verify clip creation, search/type filters, per-book draft retention, return-action UI, and visible capture controls at 340-pixel mini-player width. Real signed-in replay timing, cross-device progress, two-account switching and upgrade preservation need the owner checks in BOOKMARKS_TEST.md. These results are a local maintainer check, not an independent audit.

### Experimental annotation sync — October 20, 2026

0.2.6 adds optional device-signed annotation requests. Normal sign-in remains bearer-only; a separate checkbox requests mac_dms signing material. Older account files deserialize without it. The signing credential uses the existing Windows-protected native account file, with no frontend/export getter. Private key and token fields are zeroized on drop; decoded keys, signing inputs and annotation response buffers use zeroizing buffers. Registration response strings are scrubbed after extracting native credentials. This is best-effort memory hygiene, not protection against same-user malware or complete memory-dump confidentiality.

Requests go only to the fixed Amazon sidecar HTTPS endpoint, with redirects disabled and sensitive authentication headers. ring 0.17.14 was already locked and is now a direct Windows signing dependency. No audio/DRM functionality was added. Payloads are bounded, validate book identity and stable annotation IDs, reject unsupported records, and merge under account-generation guards. Individual sends preflight a fresh remote version, retain durable unknown-outcome intents, and require read-back verification. Unknown remote clip metadata/ranged note writes are refused rather than replaced. No automatic sync, bulk publication or cloud deletes are exposed.

Conflicts compare content against a last common baseline without assuming that a clock timestamp proves which device is newer. Both visible versions are reviewed; stale conflict choices are rejected, and accepting Audible preserves PC edits as another local item. Missing remote records and 404s never erase local notes. Local removals leave encrypted cloud-ID suppression markers. SQL savepoints roll back partial merges. Local fingerprint matching is indexed in memory once per merge to avoid rescanning the whole collection for every remote record.

Validation: 126 frontend and 111 optimized native tests pass, including ordinary/opt-in registration bodies, an independently generated ephemeral RSA key with signature verification, header sensitivity, rollback, uncertain-write reconciliation, deduplication, deleted-row ID reuse, unsupported metadata and XML escaping. Synthetic browser checks verify the new sync controls and conflict resolution preserving a separate PC copy. No real account data or signing keys were inspected. A cargo-audit run against the cached advisory database reports no vulnerability findings, with the previously documented proc-macro-error/glib informational warnings unchanged; this is not a claim of a fresh online audit.

Real service authorization, account reconnection, mobile visibility, remote text-update semantics, interruption recovery and phone OS differences remain unverified. The service has no verified atomic conditional-write contract here, so a phone edit can race between preflight and POST. This is a controlled test beta; use disposable items and the stop conditions in ANNOTATION_SYNC_TEST.md before broader release. A separate design is required for automatic operation and remote deletion.
