# Audible/mobile annotation sync — 0.2.6 test build

This is a controlled test of an unofficial API, not a verified automatic sync release. Use one owned US-marketplace title and disposable test items. Use the same account and edition on both devices. Pause playback first and note your listening position, because playing clips changes Audible's real timeline.

## 1. Preserve your data and connect

Save drafts and export important notes before testing. Install 0.2.6 over the existing app; do not erase data or delete encryption keys. Older connections have no device-signing credentials. Use Account → Sign out, check **Enable Audible annotation sync (experimental)** beside Sign in, then complete normal Amazon sign-in. Your account's local saved items should return after library sync. The checkbox is optional; ordinary sign-in remains available without it.

**Pass:** library loads, old notes remain, and the saved-items sync panel enables **Refresh from Audible**. If the panel still asks you to reconnect, stop and report the visible message only. Do not paste sign-in URLs, account files or full browser logs.

## 2. First read: phone → PC

On mobile, create a new 30-second clip with the note **LL mobile test A**. Confirm it is saved. In the PC app, select that book under **Bookmarks, clips & notes**, then choose **Refresh from Audible**.

**Pass:** the clip/note appears with the correct position, end and text. Audible may expose a clip's associated bookmark and note as separate records; this is different from importing the same cloud ID twice. Some mobile-only fields may not be returned by the service.

**Stop here if you get HTTP 401/403, unsupported annotation, or a persistent 404.** A 404 is treated as ambiguous, not as proof of an empty library. Create a fresh mobile test item, allow time for it to sync, and try a read again. Do not erase data or repeatedly send writes to work around it. Share only the status/error text and whether phone creation succeeded.

## 3. Repeat-read deduplication

Refresh three times, then restart the PC app and refresh again. Existing cloud records should retain their rows; the total must not increase on each refresh. Existing local-only notes must stay. Dates on imported items reflect their local save/import time, not a verified mobile creation time.

## 4. PC bookmark → phone

Create a new **unlabeled** PC bookmark at a position without another bookmark. Choose **Send to Audible** on that item. It should become **Verified on Audible at last check** only after a fresh server read returns it. Open the phone's bookmarks/clips list, refresh or leave/reopen that book, and confirm the position. Send/refresh again should not create a duplicate.

For an older placeholder bookmark whose text is literally Bookmark, edit it and save with an empty label first. Custom bookmark labels are local-only; publish a standalone note if you want text on mobile.

## 5. PC clip and note → phone

Create a 30-second PC clip with **LL desktop test B**, then send it. Verify its start, end, title/note and replay on the phone. Separately create **LL desktop note C** and send it. This is a metadata sync test; no audio file is transferred. Mobile-compatible outgoing clips are limited to 5–44 seconds. Longer local clips remain available on the PC but should be rejected for sending without changing mobile data.

## 6. Edits in each direction

Change a test note/clip's text on mobile, then refresh the PC. If the PC copy was unchanged, the mobile text should update it. Next edit the imported item's text on the PC without changing its times, then choose **Send edits to Audible** and verify on mobile. Avoid simultaneous mobile edits during this send: preflight checks are not an atomic server-side compare-and-swap.

Moving a synced item's start/end is not published as a destructive delete-and-create. The send should be refused until the situation is reviewed. Ranged Audible notes with different start/end offsets are read conservatively and are not supported for writing.

## 7. Conflict recovery

First refresh a synced test note. Edit it differently on the PC and mobile, without sending the PC edit. Refresh the PC. Both versions should be shown as a conflict; neither should be silently overwritten. **Use Audible; keep PC copy** should adopt the mobile version while retaining your PC edits as a separate local item. **Keep this PC version only** should retain the local version and leave mobile unchanged. A later refresh must not erase that local copy.

## 8. Removal behavior

**Remove PC copy** is deliberately local-only. Remove a disposable imported item and refresh: it should stay hidden on this PC while still existing on mobile. Delete a different disposable test item on mobile, then refresh the PC. Its PC copy should remain and be marked missing remotely if the service returns a complete list. A 404 or failed read must never cause deletions. This build sends no cloud deletion requests and does not automatically purge companions created by Audible.

## 9. Offline and interrupted sends

Disconnect before a refresh/send. Existing data should remain intact and an error should appear. If a send may already have reached Audible, the item should become **unverified**, with blind retries blocked. Reconnect, inspect mobile and refresh. If the exact item is returned, it should become verified without another create. If it remains unresolved, wait and report the error rather than repeatedly sending. Keep the PC copy/export. Do not manipulate the database to clear the marker.

## 10. Sign-out and restart boundaries

Sign out while a read/send is running, then sign in again. Responses must not populate a different account, and queued work must not use that account's credentials. An already-dispatched request may still finish on the original account; check there before retrying. If you have another test account, confirm its books/notes are separate. Restart and confirm links/unknown outcomes survive. Confirm normal library playback, mini player and progress recovery still work.

## Report results

For each step, report Pass/Fail, phone OS, app version, and the visible status text. Do not share credentials, signing keys, signed-in URLs, raw sidecar responses, notes exports or private account files. Diagnostic reports exclude annotation text and credentials. Before wider release we need successful real-device read/create/text-edit/conflict/restart tests, then a separate design and test pass for automatic refresh, retries and cloud deletions.
