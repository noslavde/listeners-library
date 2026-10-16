# Browser request context audit · 2026-10-14

## Current finding and website-session experiment

Changing the device token to `Authorization: Bearer` produced authentication rejection. That experiment was wrong for the device-token API: upstream's current change notes explicitly replace the legacy header with `x-amz-access-token` (https://github.com/mkb79/Audible/releases/tag/v0.12.0). No device-token API header is changed now.

Further inspection of the official browser entry bundle found its actual initialization chooses `AUDIBLE_COOKIE` and routes playback through `https://www.audible.com/audible-api`. The SDK's bearer branch belongs to a different Login with Amazon authentication component. Copying that header did not reproduce the website's authentication flow.

The new comparison therefore obtains the account's website cookies through the upstream-documented refresh-token exchange (`https://www.amazon.com/ap/exchangetoken/cookies`, requested token type `auth_cookies`). It requests both the streaming grant and DRM license through the fixed Audible website proxy, using the same short-lived in-memory website session. These are fresh account-owned cookies, not imported browser cookies. They stay in Rust memory, are marked sensitive, are scoped to US Amazon/Audible SSO and are never sent to API/CDN/loopback callers or written to disk. Only two fixed selected-title playback operations are permitted. Disconnect/end/drop removes the in-memory session.

This path still needs live validation: the session exchange, website proxy and resulting CDM license may independently fail. No new account login is intended. No challenge bytes, security levels or device signatures are changed. Metadata, library and progress APIs retain their existing device-token connection. The application still renders and controls its own player rather than loading Audible's player page.

Public initialization source: https://m.media-amazon.com/images/S/sash/iYiSqBHCzp1DX7e.js?awtFilename=@amzn/audible-web-player-client-assets/index.esm.js . Website-cookie exchange parameters follow the existing upstream `src/auth/cookies.rs` reference, documented in THIRD_PARTY_NOTICES.md.

Restart Tauri and use a fresh Chrome comparison link. The log should say **website session route** and **Audible website session · cookies retained in native memory**. Copy the resulting log. Do not retry the legacy bearer-header experiment.

## Earlier experiments

The ordinary Chrome comparison reproduced the same HTTP 400 / untrusted-client classification as WebView2. This establishes a shared-flow failure. It does not establish that either runtime fails DRM verification.

Unauthenticated inspection of the official public player page and its public SDK identified two fields our DRM transport omitted:

- The SDK's DRM request filter sends `Client-ID` using the configured application identifier. The public desktop web-player configuration uses `WebPlayerApplication`.
- It includes `license_id` when supplied by the streaming grant. `configureDrmForAsset` passes the current content license's ID to that request filter.

Our native DRM request now supplies that public browser-protocol header and carries the current grant's bounded license ID forward. The ID stays only in the native session, is never accepted as a browser request parameter, and is omitted from telemetry/logs. Only its presence is reported. The CDM challenge is forwarded unchanged after base64 encoding. Existing account-token authentication remains in the backend; no browser cookies, alternate device keys, content keys, or copied private client certificates are used.

This is a request-parity correction that still needs live account validation. The public SDK can use cookie or bearer authentication through different service configurations; our API registration/authentication path remains a possible difference. Do not infer from `untrusted client` alone that the CDM, its version, VMP, or Tauri is unsupported. The log label now keeps this cause unconfirmed. Explicit unsupported-CDM-version or VMP messages remain separately classified.

## Sources inspected

- Official public player: https://www.audible.com/webplayer
- Public Audible SDK asset referenced by that page: https://m.media-amazon.com/images/S/sash/2MvzPoIEsApCIxi.js?awtFilename=@amzn/audible-web-player-client-assets/audible-sdk/index.esm.js

The asset was read as source data without executing it. No user account, cookies, tokens or owned media were accessed during this audit. Public downloaded source is research-only under ignored `work/browser-protocol/`; it is not bundled with this application.

## Next test

The subsequent Chrome test confirmed both client header and grant ID were supplied, but still returned the same HTTP 400. The DRM request now also follows the public SDK's token presentation: `Authorization: Bearer …` instead of `x-amz-access-token`. Only the DRM endpoint changes; library and streaming-grant requests keep their existing authentication. Both header forms are marked sensitive in the HTTP client. The same account token is used in Rust and is not returned to browser JavaScript. The log reports only the authentication method. This is a targeted protocol comparison, not evidence that the rejection was caused by the previous header. No automatic fallback retries or credential/session-cookie imports were added.

Restart Tauri, create a fresh browser comparison link for the same title, and test Chrome again. Confirm the log reports the browser Client-ID header and whether the grant license ID is attached. Copy the resulting comparison log. No further account reconnection or runtime switch is needed for this test.
