> Update: a Shaka-based direct streaming prototype is implemented. Read DIRECT_STREAM_TEST.md for current test steps and limits. The feasibility-stage notes below are historical.

# Standalone playback direction

The user has explicitly selected a fully API-driven client rather than controlling Audible's Cloud Player. The default UI no longer launches the official player or presents playback web sign-in/visible-page scraping. Selecting a real title selects local/API metadata only. The old wrapper implementation remains in source for reference; it is not the default playback adapter.

## What works and what is still missing

API authentication, library sync, metadata, chapter lookup and progress operations remain independent of the old wrapper. Direct audio playback is **not implemented**. The standalone adapter reports unavailable audio controls rather than playing a sample or simulating a real owned title. The mini-player remains designed for a shared engine but is disabled until the standalone engine produces available playback state.

The next necessary test is in **Check direct playback** after selecting a library title:

1. Probe the app's own secure-context Encrypted Media Extensions support for temporary encrypted AAC sessions with Widevine/PlayReady. This is a capability check, not a license-key extraction tool. A rejected configuration is reported as unavailable for that configuration, not proof that all DRM support is impossible on the device.
2. If a compatible runtime is exposed, make an authenticated `Streaming` content-license request using that same API account and only the matching DRM types. No official player page, web cookies, audio download, voucher decryption or imported CDM/device keys are involved.
3. Display only grant status, reported DRM type and presence of content/license fields. No signed content URLs, license bytes, account tokens or raw API responses are returned to React or logged.

A granted content request is not playable audio by itself. A complete protected playback implementation still needs manifest/source loading, media buffering, the platform-generated license challenge and provider-authorized response exchange, duration/seek behavior, chapter mapping and progress reconciliation. A player such as Shaka can manage MSE/EME, but it still requires the provider-specific authenticated license integration. The current probe does not implement or certify that pipeline.

If the runtime is unavailable, evaluate a runtime/platform with a compatible approved DRM implementation. If the service rejects the grant, inspect the HTTP status and registration/auth protocol, without logging secrets. The original no-DRM-bypass requirement remains in effect. Tools that decode protected content to an unprotected file are a different scope and are not added here.

## Privacy and auth

The API login and old Cloud Player login were separate sessions; API registration did not authenticate the old window. That was a source of confusion and is removed from the default flow. API credentials stay in Windows Credential Manager and Rust. A user-initiated streaming grant request may create a streaming entitlement/session on Audible's service; it is not a local-only capability test. A failed/unsupported local DRM probe does not contact the license endpoint.

References: [Audible API protocol](https://audible.readthedocs.io/en/latest/misc/external_api.html), [Shaka DRM configuration](https://github.com/shaka-project/shaka-player/blob/main/docs/tutorials/drm-config.md).
