# Metadata and official playback research

Reviewed October 13, 2026. Sources are upstream documentation, not a guarantee of compatibility.

- [audible-rs README](https://github.com/mkb79/audible-rs): alpha, Rust library/CLI, Windows native core commands, library sync/export, series, and separate authentication. JSON commands commonly return `{error,warnings,result}`. The resident agent is Unix-only. Do not depend on its private SQLite schema or install its download/decryption path into this app.
- [audible-rs security](https://github.com/mkb79/audible-rs/blob/main/SECURITY.md): authentication material includes signing keys, tokens and cookies. None belong in repository files, logs or frontend storage.
- [Tauri WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html): initialization scripts, navigation callback, independent windows. Remote initialization timing varies; actual official-page behavior requires testing.
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/): grant local UI permissions only; never grant remote pages filesystem or shell access.

## Reproducible metadata experiment

On an approved development machine, independently review and pin an audible-rs release and checksum. Do not pipe a remote installer into a shell. Run `audible account login --help` and follow its external-browser sign-in options; choose encrypted auth storage and keep auth material outside this repo. The app does not request or read that auth file.

Run `audible library sync`, then inspect `audible library export --help` for the pinned version's JSON output flags. Import only the resulting metadata JSON. A dry prototype can instead use the normalized example in README without any account. Confirm a known owned ASIN, duration and series survive normalization. Redact personal metadata before sharing logs or fixtures. Command flags and export shape can change in this alpha tool.

Direct audible-rs integration is deferred until its public authentication/API surface and secure storage are audited. An app-owned auth provider must keep signing/refresh secrets in Rust with OS-protected storage, send only metadata to React, allow revocation, and request only library/catalog endpoints. Do not equate Cloud Player web cookies with mobile API credentials.

No evidence yet demonstrates signed-in Cloud Player media control inside this project's WebView2 runtime. The official-page media element approach is an explicit hypothesis. Cross-origin frames, CSP, autoplay restrictions, protected content policy, or page changes may break it. Prefer declared unavailable controls over guessing internal Audible APIs.
