# Getting started with Listener’s Library 0.2.7 beta

An unofficial companion for Audible US on Windows 10/11 x64. You need your own Audible account, internet and access to the books you play. There are no offline downloads. The app is not affiliated with Audible or Amazon.

## Install

On the GitHub release page, expand **Assets** and download the file ending **Windows-x64-Setup.exe**. Do not download Source code ZIP/TAR files. Close all copies before installing. You do not need Node, Rust or developer tools. Setup can download Microsoft's WebView2 runtime.

This installer is unsigned. Windows may report an unknown publisher or your device policy may block it. Do not disable Windows protections. Ask the maintainer for a supported option if blocked. A private repository requires access to download its releases.

## Sign in and play a book

1. Open Listener’s Library and open **Settings → Sign in**. Enter your password and verification codes only on the official Amazon forms.
2. Your library loads automatically. **Library account connected** means the library connection is saved; Amazon may still request website verification before audio can play.
3. Choose **Listen in mini player**. The official Audible window opens the title or searches your website library for it. If asked, sign in or click **Listen now** beside the highlighted title.
4. When the book starts, the compact player opens and the other windows hide. Its **↗** button restores the library; audio continues.

You can browse another series without interrupting playback. **Explore** groups books already in your library; it is not the complete Audible store catalog. **Set as current** pins a book until you change it. **Continue listening** includes meaningful listening in this app, not every book you briefly tested; its How this works menu explains the policy.

## Find your way around

Use **Library**, **Bookmarks & notes**, **My reviews**, and **Settings** at the top. Playback continues when you change sections. Account actions, privacy, diagnostics and playback help are under **Settings**. Your unsaved note and review drafts survive section changes, but not sign-out or closing the app. Save important work before updating.

The main window player has the same controls as the mini player, including chapter navigation, saved moments, sync, sleep and history. Its arrow opens the mini player; the mini player’s arrow returns to the library.

In **My reviews**, select a book, enter an optional **0.0–5.0** rating in **0.1** steps and/or write a review, then choose **Save review**. A blank rating means unrated; **0.0** is a rating. Clear rating removes only the draft rating; save to apply it. Remove review asks before deleting the saved local rating and text. These stay encrypted for this account on this PC and never sync to Audible. Library cards show your rating; **Sort → My rating · highest first** puts unrated books last. Reviews do not export with bookmarks.

## Controls and notes

- Mini player: play/pause, 15-second skips, seek bar, speed and app volume. **Previous restarts the chapter**; another click within **three seconds** steps to the preceding chapter. Further quick clicks continue backward. Next goes to the following chapter. **Chapters** keeps the full dropdown. **History**, **Sleep** and **More** reveal additional controls.
- Click the small book cover to open a larger, resizable artwork window. Close it with **Close ×**, the window's X or Escape; playback continues. Drag the title/author area to move the mini player.
- In the app: Space plays/pauses; Left/Right skip; B saves a bookmark; M switches mini player/library. These shortcuts leave focused inputs and buttons alone.
- Elsewhere in Windows: Ctrl+Alt+Space, Ctrl+Alt+Left/Right, Ctrl+Alt+B and Ctrl+Alt+M. Other apps may occupy a shortcut. Headset/media keys also work when Windows selects this app's media session.
- Open the **Bookmarks & notes** section. Select a book to read its notes without playing it. Play that book to add a timestamped note. Drafts stay with their book while the app is open, but are cleared on sign-out or closing the app. Edit/Delete act on saved notes; deletion asks for confirmation.
- To back up notes, expand the export section, type **EXPORT NOTES**, choose **Choose export file…** and select a private location. The JSON file is readable, unencrypted text. It contains notes/book identifiers, never login tokens or the encryption key. A cloud-synced destination may upload it. There is no automatic note import in this beta.

## Optional bookmark, note and clip sync

This beta offers explicit annotation reads/sends, not automatic background sync. Save drafts and export important notes first. If your existing connection lacks support, sign out, select **Enable Audible annotation sync (experimental)** beside Sign in, and reconnect through Amazon. Under **Bookmarks, clips & notes**, select a book and choose **Refresh from Audible**. Start with a disposable clip created on mobile. If that read fails, stop and follow [the round-trip test checklist](ANNOTATION_SYNC_TEST.md).

**Send to Audible** sends an individual new item; **Send edits to Audible** supports eligible same-position text edits after checking the remote version. A fresh server read must verify a send before it is marked verified. Verify on your phone separately. For compatibility, send unlabeled bookmarks, point notes, and clips from 5 to 44 seconds. Moved ranges and ranged-note writes are refused. Avoid editing the same item on mobile during a send; the API does not offer a verified atomic conditional-update contract.

When both versions changed, the app shows the Audible version alongside your PC item. **Use Audible; keep PC copy** preserves your edits as a separate local item before adopting Audible's version. **Keep this PC version only** keeps your local version without changing Audible. **Remove PC copy** is local-only; it never sends a cloud delete. Unverified sends block blind retries. Refresh and check mobile before further action. See the checklist for restart, deletion and interruption expectations.

## Saved moments

For saved moments, **◇** in the mini player adds a bookmark, **✂** saves the preceding 30 seconds (less near the beginning), and **☷** opens **Bookmarks, clips & notes** in the library. Filter or search your items and edit their text. New clips have editable start/end times in `hours:minutes:seconds`, up to five minutes apart. Optional notes can describe a bookmark or clip. These items are encrypted locally; the optional annotation-sync controls above can read and send eligible items explicitly.

**Play clip** moves the official player's timeline and pauses at the saved end. **Return to listening** seeks back to the position before your first replay, confirms the player's new position and records that choice locally. It does not resume a paused player. Audible may sync the real timeline while you replay, so return before closing if you want to keep your original spot. **Go to bookmark** also confirms and saves your deliberate position choice. See [saved-items testing](BOOKMARKS_TEST.md).

## Progress across devices

Audible's official player handles account progress. The companion separately saves dated PC checkpoints from confirmed listening, rather than from a newly loaded or paused player. A cloud check time is not a listening time. On opening a book, a difference of more than 15 seconds from the saved PC checkpoint pauses startup and opens **History**. Choose **Resume PC** or **Use Audible**. Our integration has no verified cloud listening time, so it does not guess which position is newer or assume the farthest position is the right one.

**History → Recent positions on this PC** lets you restore a dated entry. It is also in **Listening history & recovery** in the main app. Checkpoints save every five seconds while listening, with a final save on pause, book switch or normal app close. The log keeps one periodic entry per minute plus pause/switch/close/explicit position choices. It is bounded to 100 entries per book and 3,000 per account; entries older than 90 days are pruned on a subsequent save. The latest checkpoint per book is retained separately. This stores small metadata records, not audio, and does not track mobile listening sessions.

On first use of this update, any existing PC checkpoint and credited listening record are preserved where available. Older PC checkpoint times may reflect player observations. Positions already overwritten in every old record cannot be reconstructed. Sign-out hides the log with the rest of the account; its database stays encrypted and isolated from other accounts.

When returning from mobile, pause there and allow Audible to save progress, then press **Sync position** in the mini player. This fetches Audible's saved position for the playing book and moves to it, forward or backward, preserving play/pause. It reports success after the official player's timeline reaches that position. If no position is available, the timeline does not fit, or loading prevents a seek, it explains the problem without guessing. Changing books or using playback controls during the check cancels it. The app does not automatically jump after idle: Audible's reported position may still be stale, so check the chapter/time before resuming.

To choose a local checkpoint instead, open **Playback help & options → Saved progress**, refresh, then choose **Resume on this PC** or **Resume Audible position**. Cross-device updates may take time.

## When something does not work

| What you see                            | What to do                                                                                                                                                    |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sign-in is still open                   | Complete Amazon verification, or use Account → Cancel sign-in. Retry when ready.                                                                              |
| Signed in but library is empty          | Wait for sync, then choose Sync library. If it fails, check internet and the visible message.                                                                 |
| Book has not started                    | Choose Open Audible window. Complete verification or click Listen now beside the highlighted book. Retry this book if necessary.                              |
| A sample or Get this title appears      | Use Playback help & options → Open Audible library, find the owned title and click its Listen now. Do not continue from the sample.                           |
| No sound                                | Check the app volume, paused state and Windows audio output, then show the official player. Its visible message may explain verification or playback failure. |
| Chapter cannot be selected              | Show the official player and try its Chapters control. Report whether official selection worked.                                                              |
| Notes look missing after sign-in/update | Do not erase data. Read recovery below.                                                                                                                       |
| Sign-out cleanup failed                 | Account access has stopped. Choose Account → Retry sign-out. Do not share the Windows session until cleanup finishes.                                         |
| App reports an encryption/key problem   | Close other copies and retry. Do not delete/replace the key or erase data. Ask for help.                                                                      |

## Updating and recovering older notes

There is no automatic updater. Download the next Setup.exe from Releases, close every copy and install it. Updates from Listener’s Library 0.2.0 keep account profiles and notes. The older Audible Desktop installation may remain separately listed: do not run it or use it to bypass the newer account boundary. Remove its shortcut to avoid confusion.

Earlier shared notes/history were not reliably linked to an account. They remain encrypted but are not automatically assigned. **Bookmarks, clips & notes → Export & older notes** lets you type **EXPORT OLD NOTES** and save old notes to a private JSON file. This may contain notes from accounts used in older versions; only do it if you own those notes and control this Windows account. It does not merge notes/history into the current account. Recovery needs the original Windows-protected key; it cannot recover a lost key or another Windows user's data.

If Amazon omits a stable account identity, the app warns that a later sign-in may open another isolated profile. Export important notes before signing out. Do not erase older stores hoping that this will fix missing data.

## Signing out, removing data and uninstalling

**Account → Sign out** stops playback, closes players and hides books/notes. Encrypted notes/history remain for that account. Sign-out also clears website sign-in; retry any reported cleanup failure.

**Privacy & local storage → Erase local app data** permanently removes retained files after confirmation and restart. Export notes first if needed. This does not remove existing exports/backups or purchases from Audible. Use Windows Settings → Apps → Installed apps to uninstall. Uninstalling alone should not be treated as signing out or erasing retained personal data.

## Reporting a problem

For an app crash, invisible window or reopening problem, open **Troubleshooting & diagnostic report**, review the report and save the JSON if you want to share it. It excludes account/book information and raw errors; nothing uploads automatically. Save it before repeated restarts replace older events. An “unclean” session is not proof of a crash. See [close/reopen checks](LIFECYCLE_TEST.md).

Closing the main library exits playback and the app. There is no system-tray mode. Closing the mini player returns to the library and keeps audio playing. Launching the shortcut again restores the running library. If an older version is stuck invisibly, end **listeners-library.exe** in Task Manager before upgrading; keep its protected data and encryption key.

Use the repository's Issues page with the app version (About & privacy), Windows version, steps, expected result and visible message. If sign-in fails, the app's **Sign-in diagnostic** contains limited connection stages and can help. Never share passwords, MFA codes, cookies, signed-in URLs, account files, encryption keys or full browser logs. For a potential security issue, follow SECURITY.md rather than posting details publicly.

This beta still needs real testing on different accounts and clean PCs. Passing automated tests does not establish that every Amazon account, WebView2 version or device policy works.
