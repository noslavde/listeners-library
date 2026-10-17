# Architecture — 0.2.1 beta

## Application and playback

React owns library filtering, owned-series grouping and local controls in the main window. A second local window is the always-on-top mini player. The borderless return button restores the library while playback continues. Filtering never navigates/reloads the player.

OfficialAdapter invokes scoped native commands. Audible's official WebView2 player handles protected audio and account playback progress. The app does not extract keys or decrypt audio. A bounded JS bridge reports typed, validated media state through intercepted navigation; stale reports disable controls. Exact HTTPS Audible/Amazon US origins are allowlisted. Provider windows have no Tauri capabilities. Local command helpers verify both the window label and local origin.

Title opening uses an already observed, sanitized launch address when possible. Otherwise launch.js finds the exact book in the official website library, searches once if needed and clicks a unique enabled Listen now control. Lookup/click deadlines are bounded; failure leaves a highlighted title for manual selection. The mini player opens only after full-title playback starts. First-use website verification can still require interaction.

## Account lifecycle

Amazon registration uses the working Android WebView PKCE flow. Password/MFA entry stays on Amazon forms. Rust exchanges and refreshes tokens against fixed HTTPS endpoints, with redirects disabled and bounded response handling. Tokens stay native and are saved/read back under Windows protection before a connection is published. Connection diagnostics contain locally defined stages, not raw response bodies or secrets.

A lifecycle generation invalidates outstanding work on sign-out. A durable marker blocks account restoration across restart before credential/browser cleanup. Cleanup closes the official page, waits for WebView2's browsing-data deletion callback and verifies cookies. A visible Retry sign-out action handles incomplete cleanup. Signing in after a sign-out retries cleanup first.

## Storage and notes

SQLCipher databases use one random Windows-user-protected key, with no plaintext fallback. Account namespaces prefer the provider's authenticated customer identity. Missing identity/legacy credentials use a registration-derived namespace; no ownership is guessed. That fallback does not guarantee continuity across fresh registrations. Generic preferences remain shared locally. Development and installed scopes are separate.

Unassigned old library.sqlite notes/history remain encrypted. There is no automatic account migration. Explicit old-note export is available to their local owner, without merging data into an account. There is no automatic import in this beta.

Notes use stable-in-place SQLite row identifiers for edits/deletes, validated book/timestamp/text and account-scoped native database access. Frontend drafts belong to a book and retain the initial typing timestamp. Note reads do not start audio. Explicit JSON export is handled by a native save dialog, checks the originating account generation again before writing and includes no tokens, cookies or key. Exported JSON is intentionally readable; the UI requires typed confirmation and explains the boundary.

## Progress and resilience

Live playback is authoritative for the current player. A cloud refresh timestamp is not a provider listening timestamp. Conflicting saved local/cloud positions are displayed separately and excluded from progress ranking; the official player chooses its resume position unless the user explicitly seeks to a saved checkpoint.

A React error boundary provides a recovery screen without logging raw exceptions. Core user docs are TESTER_GUIDE.md and PRIVACY.md; research documents describe retired experiments, not production features. Automated dependency/test/build gates and remaining live checks are documented in RELEASE.md and SECURITY_REVIEW.md.
