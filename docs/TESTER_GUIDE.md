# Getting started with Listener’s Library 0.2.2 beta

An unofficial companion for Audible US on Windows 10/11 x64. You need your own Audible account, internet and access to the books you play. There are no offline downloads. The app is not affiliated with Audible or Amazon.

## Install

On the GitHub release page, expand **Assets** and download the file ending **Windows-x64-Setup.exe**. Do not download Source code ZIP/TAR files. Close all copies before installing. You do not need Node, Rust or developer tools. Setup can download Microsoft's WebView2 runtime.

This installer is unsigned. Windows may report an unknown publisher or your device policy may block it. Do not disable Windows protections. Ask the maintainer for a supported option if blocked. A private repository requires access to download its releases.

## Sign in and play a book

1. Open Listener’s Library and choose **Sign in**. Enter your password and verification codes only on the official Amazon forms.
2. Your library loads automatically. **Library account connected** means the library connection is saved; Amazon may still request website verification before audio can play.
3. Choose **Listen in mini player**. The official Audible window opens the title or searches your website library for it. If asked, sign in or click **Listen now** beside the highlighted title.
4. When the book starts, the compact player opens and the other windows hide. Its **↗** button restores the library; audio continues.

You can browse another series without interrupting playback. **Explore** groups books already in your library; it is not the complete Audible store catalog. **Set as current** pins a book until you change it. **Continue listening** includes meaningful listening in this app, not every book you briefly tested; its How this works menu explains the policy.

## Controls and notes

- Mini player: play/pause, 15-second skips, seek bar, speed and app volume. The current chapter appears above the timeline; its arrows go to the previous/next chapter. **Chapters** keeps the full dropdown, while **Sleep** and **More** reveal additional controls.
- Click the small book cover to open a larger, resizable artwork window. Close it with **Close ×**, the window's X or Escape; playback continues. Drag the title/author area to move the mini player.
- In the app: Space plays/pauses; Left/Right skip; B saves a bookmark; M switches mini player/library. These shortcuts leave focused inputs and buttons alone.
- Elsewhere in Windows: Ctrl+Alt+Space, Ctrl+Alt+Left/Right, Ctrl+Alt+B and Ctrl+Alt+M. Other apps may occupy a shortcut. Headset/media keys also work when Windows selects this app's media session.
- Open **Notes & backups** in the library. Select a book to read its notes without playing it. Play that book to add a timestamped note. Drafts stay with their book while the app is open, but are cleared on sign-out or closing the app. Edit/Delete act on saved notes; deletion asks for confirmation.
- To back up notes, expand the export section, type **EXPORT NOTES**, choose **Choose export file…** and select a private location. The JSON file is readable, unencrypted text. It contains notes/book identifiers, never login tokens or the encryption key. A cloud-synced destination may upload it. There is no automatic note import in this beta.

## Progress across devices

Audible's official player handles account progress. The companion also saves checkpoints on this PC. A cloud check time is not the time you last listened. If saved positions differ, the card shows both rather than declaring one newer. Starting a book uses the official player's resume behavior; it does not force the card's local checkpoint.

When returning from mobile, pause there and allow Audible to save progress, then press **Sync position** in the mini player. This fetches Audible's saved position for the playing book and moves to it, forward or backward, preserving play/pause. It reports success after the official player's timeline reaches that position. If no position is available, the timeline does not fit, or loading prevents a seek, it explains the problem without guessing. Changing books or using playback controls during the check cancels it. The app does not automatically jump after idle: Audible's reported position may still be stale, so check the chapter/time before resuming.

To choose a local checkpoint instead, open **Playback help & options → Saved progress**, refresh, then choose **Resume on this PC** or **Resume Audible position**. Cross-device updates may take time.

## When something does not work

| What you see | What to do |
| --- | --- |
| Sign-in is still open | Complete Amazon verification, or use Account → Cancel sign-in. Retry when ready. |
| Signed in but library is empty | Wait for sync, then choose Sync library. If it fails, check internet and the visible message. |
| Book has not started | Choose Open Audible window. Complete verification or click Listen now beside the highlighted book. Retry this book if necessary. |
| A sample or Get this title appears | Use Playback help & options → Open Audible library, find the owned title and click its Listen now. Do not continue from the sample. |
| No sound | Check the app volume, paused state and Windows audio output, then show the official player. Its visible message may explain verification or playback failure. |
| Chapter cannot be selected | Show the official player and try its Chapters control. Report whether official selection worked. |
| Notes look missing after sign-in/update | Do not erase data. Read recovery below. |
| Sign-out cleanup failed | Account access has stopped. Choose Account → Retry sign-out. Do not share the Windows session until cleanup finishes. |
| App reports an encryption/key problem | Close other copies and retry. Do not delete/replace the key or erase data. Ask for help. |

## Updating and recovering older notes

There is no automatic updater. Download the next Setup.exe from Releases, close every copy and install it. Updates from Listener’s Library 0.2.0 keep account profiles and notes. The older Audible Desktop installation may remain separately listed: do not run it or use it to bypass the newer account boundary. Remove its shortcut to avoid confusion.

Earlier shared notes/history were not reliably linked to an account. They remain encrypted but are not automatically assigned. **Notes & backups → Export notes or recover notes from older versions** lets you type **EXPORT OLD NOTES** and save old notes to a private JSON file. This may contain notes from accounts used in older versions; only do it if you own those notes and control this Windows account. It does not merge notes/history into the current account. Recovery needs the original Windows-protected key; it cannot recover a lost key or another Windows user's data.

If Amazon omits a stable account identity, the app warns that a later sign-in may open another isolated profile. Export important notes before signing out. Do not erase older stores hoping that this will fix missing data.

## Signing out, removing data and uninstalling

**Account → Sign out** stops playback, closes players and hides books/notes. Encrypted notes/history remain for that account. Sign-out also clears website sign-in; retry any reported cleanup failure.

**Privacy & local storage → Erase local app data** permanently removes retained files after confirmation and restart. Export notes first if needed. This does not remove existing exports/backups or purchases from Audible. Use Windows Settings → Apps → Installed apps to uninstall. Uninstalling alone should not be treated as signing out or erasing retained personal data.

## Reporting a problem

Use the repository's Issues page with the app version (About & privacy), Windows version, steps, expected result and visible message. If sign-in fails, the app's **Sign-in diagnostic** contains limited connection stages and can help. Never share passwords, MFA codes, cookies, signed-in URLs, account files, encryption keys or full browser logs. For a potential security issue, follow SECURITY.md rather than posting details publicly.

This beta still needs real testing on different accounts and clean PCs. Passing automated tests does not establish that every Amazon account, WebView2 version or device policy works.
