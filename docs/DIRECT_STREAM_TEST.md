# Direct protected-stream playback

The user's runtime test confirmed support for temporary encrypted AAC sessions and a Widevine `Granted` response. The earlier probe incorrectly checked only `content_metadata.content_url`: for this Widevine protocol, `content_license.license_response` carries the DASH manifest URL, not an already usable content-key payload. The parser now recognizes that URL without displaying it in diagnostics.

## Current test

1. Restart Tauri and confirm the API account is connected.
2. Select the same owned title and choose **Load direct playback**.
3. Wait for **Direct protected stream loaded · press Play**, then press Play using the audio control or persistent controls.
4. Test a later chapter, seek, speed and pause. Open **Mini player ↗**, minimize the library, and confirm playback and controls continue.
5. If loading fails, report the stage message and error code shown. Do not copy developer-console request URLs, license bytes, tokens or signed CDN URLs.

## Implementation

Shaka handles DASH buffering and standard EME sessions with the installed Widevine runtime. Rust requests the API streaming grant, keeps its session metadata in memory, fetches encrypted media resources and forwards platform-generated license challenges to the fixed Audible DRM-license endpoint. Encrypted license messages return to the platform CDM through Shaka. No content keys are extracted, no external CDM/device credentials are imported, and no DRM-free audio file is produced.

Media requests are session-scoped, HTTPS-only and limited to the grant's exact origin. Redirects to other origins are refused. If a real manifest requires another CDN origin, the loader will stop pending review rather than broadly opening network access. API tokens remain in Rust; a token is sent on media requests only when the grant host belongs to audible.com, and same-origin redirects are enforced. Each media response is bounded to 32 MB; the prototype transfers chunks over local IPC and may need performance tuning.

Signed manifest/resource URLs necessarily exist transiently inside Shaka's local networking objects so relative segment URLs resolve correctly. They are not placed in React UI state, persisted to the library database or intentionally logged by app code. They are private bearer-style data: do not share developer-console network dumps. CDN traffic is to the provider resource, not a separate proxy service. The code suppresses license-exchange logging and displays Shaka error codes without dumping their request-data payloads.

Commands from main and mini-player target the same audio element in the main window. Native snapshots and local checkpoints use the common PlayerHub. Chapter selection uses Shaka's stream timeline rather than an Audible page's media element. Cloud resume comes from the streaming grant if provided; manual progress upload remains separate and automatic account writes remain disabled. Closing/changing the selected title tears down the old player; API disconnect invalidates streaming and closes playback in the renderer.

## Validation limits

The build and native unit tests cover manifest parsing, URL/session guards and protocol normalization; they do not exercise a real CDM license exchange. The user has confirmed runtime capability and the first grant only. Audible may require a different license challenge wrapper, authenticated CDN handling, DRM provisioning or additional origins. A granted content request and available key system are necessary evidence, not proof of successful audio playback. This prototype must be tested on the account before it is described as working end to end.
