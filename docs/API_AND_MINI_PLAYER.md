# API library and always-on-top mini player

For the current account test, follow [NEXT_TEST.md](NEXT_TEST.md). The normal workflow is **Sign in → Sync library → Listen in mini player**. Full-book playback triggers the mini player automatically; **Back to app ×** restores the library without stopping audio. Both larger windows hide while the mini player is open.

## Library and progress

Library sync runs in Rust and fetches continuation-token pages before updating the local SQLite cache. The normal UI no longer offers JSON import, visible-page scraping or API registration-profile selection. Existing cached libraries remain readable.

Chapter/cloud metadata loads when selecting a title. **Playback help & options** holds refresh and explicit cloud/local resume controls. Missing or unsupported positions are reported as unavailable rather than zero. Local checkpoints are recorded roughly every ten seconds while valid title-matched telemetry is received; startup zero does not erase a saved nonzero checkpoint.

The official player retains its cloud-resume behavior. Read-only API lookup and an optional manual upload test are separate features; they do not establish two-way sync. Automatic uploads remain off. See [position sync](POSITION_SYNC_TEST.md).

## Security boundaries

Amazon OAuth uses S256 PKCE and an exact HTTPS callback. API tokens remain in Rust and Windows Credential Manager, never React, SQLite, project files or logs. API requests use fixed provider hosts and disable redirects. Official playback and account sign-in share a persistent local WebView2 profile; Audible can still request verification.

**Sign out** removes this app's API credentials and remembered website cookies. Cached books, notes and checkpoints remain. This does not deregister the device from Amazon. The app is a single-account prototype; local metadata and notes are not encrypted. See [privacy](PRIVACY.md).

## Runtime limits

Automatic new-title selection depends on Audible's live library markup. It retains search, highlighting and a manual Listen now fallback. Samples do not trigger automatic mini-player opening. If the official player refuses a Play request, it is revealed for a manual click. Do not close the official player or exit the whole app while listening.

The frontend build and automated tests pass. The user has confirmed official signed-in playback and remembered title launches. The new automatic mini-player handoff, hidden playback, chapter behavior and cross-device progress require live testing. Standalone encrypted-streaming code remains for reference but is absent from the normal UI and packaged browser pages.
