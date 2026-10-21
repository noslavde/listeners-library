# Listener’s Library 0.2.6 — experimental Audible annotation sync

- Added an optional sign-in mode that requests device-signing credentials for annotation access. Existing bearer-only login stays available. Keys/tokens remain Windows-protected in the native account store and are never exposed to website JavaScript, note exports or diagnostics.
- **Refresh from Audible** reads a selected book's bookmarks, notes and clips into its encrypted account cache, using cloud IDs for repeat-read deduplication. It never imports or writes last-heard records as annotations.
- **Send to Audible** explicitly publishes one new item, or an eligible same-position text edit after checking the current remote version. A successful HTTP response is not enough: a fresh read must verify the result. No automatic bulk upload or background synchronization is enabled.
- Added local/verified/PC-changed/conflict/missing/unverified states. Conflicts preserve both versions for review; accepting Audible retains the edited PC version as a separate copy. A durable unknown-write marker prevents blind duplicate sends after network failures or restarts.
- Removing a PC copy is local-only and stays hidden on later reads. Items missing remotely remain locally recoverable. HTTP 404, malformed responses and failed reads never imply that local data should be erased.
- Sending supports unlabeled bookmarks, point notes and 5–44 second clips. Local clips can still be longer. Moved ranges, ranged-note writes, cloud deletions and simultaneous-write guarantees are not implemented. Read-before-write checks are not an atomic server-side conditional update.

Local Windows test beta. Live account/device compatibility is unverified. Follow [the mobile round-trip checklist](ANNOTATION_SYNC_TEST.md), starting with a read of a fresh disposable mobile clip. Stop if authentication or the first read fails. [Getting started](TESTER_GUIDE.md).
