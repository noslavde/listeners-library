# Manual Audible position upload test

The documented unofficial endpoint is `PUT /1.0/lastpositions/{asin}` with a JSON body containing `asin`, `acr` and integer `position_ms`. The implementation uses the existing protected API token, fixed HTTPS host and redirects-disabled client. It does not request audio, a DRM license or decryption material. Source: [Audible API documentation](https://audible.readthedocs.io/en/stable/misc/external_api.html#misc).

## Test on your account

1. Restart the desktop app and connect the library API. Sync your library so the selected title has runtime metadata. Use the same account for API and official playback.
2. Open a title and play a little past its existing cloud position. Pause, then wait a second for the controls to show paused.
3. Choose **Prepare position upload**. The app reads the cloud position and content reference from metadata. It checks that the media duration is consistent with the cached whole-book runtime. If the cloud position is ahead, resume cloud first. If the reference is absent or the timeline cannot be checked, no write is offered.
4. Review the book title, existing cloud time and proposed paused time. Choose **Send position to Audible** to perform one account write, or **Cancel** to invalidate the preview. Previews expire after 60 seconds and are single-use.
5. The app re-reads cloud progress before sending. Any position/content-reference change since preview stops the upload. A changed title, resumed player, stale telemetry or changed account also stops it.
6. After a successful HTTP response, the app re-reads metadata. It reports a matching read-back separately from a merely accepted request. A network error may leave the outcome unknown; do not retry blindly—check cloud first.
7. Open the same title in Audible mobile and the ordinary web player; confirm both offer the same position. Report only the result or a redacted error. No credentials, callback URLs, tokens or account exports are needed in the chat.

## Limits

No automatic uploads are enabled. The agent has not performed an account write or confirmed live endpoint authentication, content-reference availability, read-back freshness or mobile propagation. A 2xx response or synthetic test is not proof of cross-device synchronization.

The unofficial API provides no verified conditional-write/ETag guarantee here. Another device can still update between the preflight read and PUT; keep other devices idle for the first test. The implementation conservatively blocks moving cloud progress backwards. Intentional rewind upload needs a separate design. The official Cloud Player can continue making its own provider-managed progress updates.

Credentials stay in Rust and Windows Credential Manager. The UI receives a one-use local preview ticket and playback times, not credentials or the raw content reference. Cancel clears the native pending preview. Local checkpoint storage remains separate from account progress.
