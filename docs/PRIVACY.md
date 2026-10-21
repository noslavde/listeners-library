# Privacy and local data — 0.2.6 beta

Listener’s Library is independent of Audible and Amazon. It has no app analytics, crash uploader or app-operated notes server. Audible/Amazon receive normal account and playback requests, optional annotation-sync requests, and their official website may collect its own telemetry. Public book covers load from Amazon image servers.

## What stays on this PC

Your library, notes, local listening history, saved positions and account-specific selections are stored in encrypted local databases. Login tokens and the database key are protected by Windows for the current Windows user. The key is random, is not in the installer and is not sent to the app interface. There is no unencrypted database fallback or developer recovery key.

Bookmarks and clips use the same encrypted account database. They contain a book identifier, saved type, start/end positions, optional text and optional local save/import time. No audio files are captured. Replaying a clip uses the official player and changes its real timeline, so Audible may sync that listening position. Return to listening seeks back explicitly. Their content is excluded from diagnostic reports. Readable notes exports include clip ranges, sync state and conflicting note versions when present; keep them private.

Optional experimental annotation sync requests a device-signing credential when you select Enable Audible annotation sync at sign-in. It is stored with the Windows-protected native account and never returned to webviews, diagnostics or note exports. Refresh reads annotations directly from Amazon's fixed HTTPS sidecar service; Send explicitly submits one selected annotation there. Redirects are disabled. The app adds no third-party sync server. There is no automatic bulk upload or periodic sync.

Cloud annotation IDs, last common versions, conflict snapshots, hidden-record markers and uncertain-write intents stay in the account's encrypted database. Removing a PC copy does not remove it from Audible; the hidden marker prevents repeat imports of that cloud ID. Items missing remotely remain locally recoverable. A request dispatched before sign-out may still finish on its original account; future-account results and credentials are isolated by generation guards. Sign-out retains encrypted sync state and blocks automatic account restoration until cleanup completes, as with other account data. Erase removes those account stores. Server copies are controlled through Audible, not app-local erasure.

The recovery log stores book identifiers, playback positions, durations, local timestamps and event labels. It contains no audio, cookies or tokens. It is limited to 100 entries per book and 3,000 per account, with entries older than 90 days pruned on subsequent saves. The most recent checkpoint per book remains separately until replaced by listening or an explicit position choice. It is not an exhaustive listening history or a record of mobile sessions.

Website sign-in is separate: Microsoft's WebView2 manages Audible/Amazon website data locally. It is not encrypted by the library database. Website password saving and autofill are disabled. Turn off **Remember website sign-in on this PC** under Privacy & local storage to use private website windows on subsequent connections. The encrypted library account is still remembered until you sign out.

Appearance and window placement include nonsensitive local settings outside the encrypted database. General player/appearance preferences are shared across accounts on the same Windows profile. Development and installed versions use separate private stores.

## Account separation and recovery

Current account libraries, notes and history have separate encrypted files. When Amazon supplies its stable account identity, returning to that account returns to the same profile. If identity is absent or an older credential has none, the app uses an isolated registration profile and warns that a later sign-in may open another profile. It does not guess identity from email or overlapping books.

Old shared notes/history remain encrypted, but are not automatically assigned to anyone. **Bookmarks, clips & notes → Export & older notes** offers an explicit old-notes export for the owner of those notes on this Windows account. It does not merge accounts or recover missing keys. The export may contain notes from accounts used in earlier versions; use it only if you own those notes and control this Windows account.

## Exports and backups

Notes are not sent anywhere automatically. You can explicitly type EXPORT NOTES or EXPORT OLD NOTES and choose a JSON file using Windows' save dialog. That file contains readable notes and book identifiers; it contains no login tokens, cookies or encryption key. The export is not encrypted by this app. Choose a private location and protect/delete it yourself. A cloud-synced destination may upload the file. This beta has no automatic note import or full portable account backup.

## Sign-out and removal

**Sign out** immediately stops account access and playback, hides books/notes and closes players. A saved sign-out marker prevents automatic reconnection after restart while credential/browser cleanup finishes. Cleanup waits for WebView2's browsing-data deletion callback and checks cookies. Failures are shown; choose **Retry sign-out** before sharing this Windows session. A new sign-in retries cleanup. Signing out retains encrypted account notes/history; it does not revoke other devices or remove purchases.

**Remove old version data** deletes known legacy files/profile after confirmation and restart; it does not delete every current encrypted account profile. Export old notes first. **Erase local app data** removes current-scope encrypted files, key, credentials and website profile; installed builds also remove known legacy stores. Export any needed notes first. Typed confirmation and restart are required. Missing/corrupted keys never silently reset an existing database.

Uninstalling alone is not signing out or an assurance of data erasure. Exports, backups, filesystem/cloud copies and forensic remnants are outside app removal. Changing Windows users, reinstalling Windows or losing its protected key may make saved data inaccessible. Do not delete the key as a troubleshooting step.

## Optional diagnostic report

The app keeps a small plaintext technical journal in a Windows-user-restricted diagnostics directory, separate from encrypted listening history. Up to 128 events from each of the last two sessions are retained. Fields are limited to predefined event codes, relative elapsed times, termination status, app version and architecture. There are no account identifiers, book identifiers/titles, notes, URLs, paths, machine/user names, raw exception messages, stack traces, memory dumps or tracking IDs. It is designed for sharing without personal content; no system can promise absolute anonymity from context you separately provide.

**Troubleshooting & diagnostic report** lets you inspect the exact report and explicitly save it through Windows' file dialog. Nothing is uploaded automatically. Exports are readable JSON; a cloud-synced destination may upload your saved file independently. Sign-out keeps this nonsensitive journal. Erase local app data removes its previous history; restarting creates a fresh session journal. Uninstall alone may retain it.

Diagnostics are best effort. They distinguish incomplete shutdown from clean exit, but cannot determine whether an incomplete session was a crash, forced stop or power failure. Some native crashes and failures before diagnostic initialization leave no error record. Saving can fail if storage is unavailable. Collect a report before repeated restarts replace older sessions.

## Security limits

Windows-user encryption is not an app-exclusive vault. Malware running as you, an administrator or a compromised operating system can access decrypted data. Files can still be copied even if their contents cannot ordinarily be read elsewhere. No claim of absolute nontransferability or forensic erasure is made. The unsigned beta has not received an independent security audit.

Remote provider windows have no local app capabilities. Native tokens are not returned to website JavaScript or React. Diagnostic messages contain bounded locally defined stages, not raw provider responses or credentials. Never attach credentials, account files, cookies, keys, signed-in URLs or full browser logs to a report. Read [Security policy](../SECURITY.md) and [validation evidence](SECURITY_REVIEW.md).
