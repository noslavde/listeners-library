# Listener’s Library 0.2.5 beta — bookmarks, clips and notes

- Replaced the old notes list with a saved-items panel: book selection, search, type filters, chronological/recent sorting, chapter context and saved dates where available.
- Added bookmarks with optional text and clips with editable start/end times. Capture the preceding 30 seconds, or choose a range up to five minutes. Notes keep a fixed timestamp while writing. Drafts stay with their book until closing or sign-out.
- Saved clips replay through the official player and pause at the end. Manual seeking/chapter selection cancels the old clip boundary. Replay moves the real timeline; **Return to listening** seeks back to the position captured before the first replay. End timing is best effort; no audio is copied, downloaded or exported.
- The mini player can save a bookmark, capture a clip, and open saved items. Ctrl+Alt+B remains a bookmark shortcut.
- Existing notes and bookmarks remain intact. New metadata stays in the account's encrypted database; there is no Audible mobile synchronization or import. Notes export version 2 includes type, clip range and optional creation date. Reports for developers still exclude all note/clip content.
- Editing and deleting validate the full previous item to reject stale changes. Account-generation guards pin queued commands to the original account. Reused legacy note IDs cannot inherit a deleted clip's details.

Local Windows test beta. Verify replay, end pausing, progress recovery, upgrades, old notes, account separation and compact layout using [the checklist](BOOKMARKS_TEST.md) before publishing. Previous close/reopen fixes and private diagnostics remain included.
