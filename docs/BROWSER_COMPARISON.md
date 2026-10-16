# Ordinary browser playback comparison

This diagnostic runs the same `StreamEngine` / Shaka configuration in Edge or Chrome instead of WebView2. Both use the same Rust streaming core for the grant, provider certificate, encrypted media and platform-license exchange. It tests whether the ordinary browser CDM is accepted by the same Audible API flow. It does not guarantee playback or change the application stack.

The current experiment uses the account's website session and Audible's website playback proxy for both grant and license, following the public browser player's initialization. Library/progress access still uses the existing device-token API. Website cookies obtained via refresh-token exchange remain only in Rust memory and are never returned to the comparison tab. See BROWSER_PROTOCOL_AUDIT.md for the evidence and earlier failed device-token experiments.

## Test

1. Restart `npm run tauri dev`. Startup now builds both frontend entry pages before starting the development server. Keep the existing Android API connection; no new Audible sign-in is needed.
2. Select the same library title. Under **Compare playback in Edge or Chrome**, choose **Create browser test link**. Embedded audio is stopped first.
3. Use **Open default browser** if your default is Edge or Chrome, or **Copy test link** and paste into one of those browsers. The link can pair once and expires for pairing after two minutes. Create a fresh link for another browser or a new attempt to pair. Once paired, that tab may reload while the local connection remains active.
4. In the browser, choose **Load browser playback**, then **Play**. Check whether time advances and actual audio is heard. If it works, test a later chapter and a 15-second skip.
5. Use **Copy comparison log** in the browser and report the result. The log labels the actual browser runtime; the desktop-host WebView2 version is diagnostic context and does not identify the browser's CDM version.
6. Use **Stop browser test** in Tauri when finished. Disconnecting the API, selecting another title, closing Tauri, or the 20-minute expiry also ends the connection. The browser detects loss of the connection and stops audio on its three-second heartbeat.

If the browser also returns the same untrusted-client rejection, changing the desktop shell alone is not justified; investigate API-to-CDM compatibility. If it plays successfully, the result narrows the failure to the embedded environment, its origin or its generated challenge. It does not by itself prove which WebView feature differs.

## Local connection boundaries

- Rust binds an ephemeral IPv4 loopback port, never a LAN interface. Exact Host checking blocks DNS rebinding. Mutating requests require the exact local Origin; no CORS access is granted.
- The one-use 256-bit pairing capability travels in the URL fragment, not in HTTP paths/query strings. The browser removes it from the address bar before pairing and exchanges it for an HttpOnly, SameSite=Strict session cookie. Keep the temporary link private while it is valid. No account access/refresh tokens are sent to the browser.
- The bridge serves only built HTML/JS/CSS assets and exposes a small typed playback protocol for the selected title. It has no login, token, library-management, notes, arbitrary API, filesystem or arbitrary-network proxy operation. Media URLs retain the existing HTTPS/same-CDN-origin restrictions. Request bodies are bounded.
- The browser receives the selected title, provider certificate, encrypted audio and platform license responses needed for protected playback. Signed media URLs exist transiently for relative segment resolution. They are omitted from logs. No private CDM keys are imported or extracted.
- Browser telemetry stays in that tab. It does not create desktop position checkpoints or upload progress. Account tokens stay in Windows Credential Manager. The test log stays in memory and can be copied explicitly.
- Starting another comparison revokes the previous connection. Explicit stop, API disconnect and timeout revoke pairing/cookies and shut down the listener. Requests already transferring can finish during graceful shutdown, within the existing native network timeout.

## Validation scope

Build and automated checks cover the shared transport, pairing, expiry/revocation, Host/Origin checks, asset traversal rejection, operation restrictions and preservation of sanitized license failures. Audible playback in a real browser still requires the user's account test. A Widevine capability probe alone is not proof of a playable license.
