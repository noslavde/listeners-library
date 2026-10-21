# Navigation, player and local reviews — 0.2.7 beta

Close the previous app before installing this local test build. Save any drafts first. Existing account and progress stores are retained. This checklist requires a signed-in Windows build and an owned book; browser previews cannot verify audio or durable account storage.

1. Open **Settings**. Confirm your account is connected and the library loads. Sign-in, sign-out, privacy and diagnostic controls now live here.
2. In **Library**, start a book. Return from the mini player with its arrow. Confirm audio continues. Switch between **Library**, **Bookmarks & notes**, **My reviews**, and **Settings**: no playback restart or position change should occur.
3. In the main player, try play/pause, ±15 seconds, speed, seeking, chapter/book progress, next chapter, bookmark, last-30-second clip, cover enlargement, sleep, history and recovery under More. Test Previous once to restart, then twice quickly to step backward. Compare with the mini player.
4. Pause on mobile and allow it to sync. Try **Sync position** from the main player. Check the reported chapter/time before resuming. This still depends on Audible's saved position; no automatic conflict resolution was added.
5. Check app volume at 0%, 50% and 100%. The breakpoint labels should align with the left, middle and right of the slider. Confirm volume and mute work in both windows. This is independent of Windows system volume.
6. In **My reviews**, select a book. Save **0.0** with no text: it should display **0.0 / 5**, not Unrated. Change to **4.7**, add text and save. Try **5.0** and reject **5.1**, negative values and more than one decimal place. A review without a rating is allowed; an empty rating and empty review are not.
7. Edit without saving. Switch sections and switch books, then return: the draft should still belong to its original book. Discard restores the saved version. Save before closing; unsaved drafts do not survive restarts.
8. Check the matching library card shows your saved score. Choose **Sort → My rating · highest first**: rated books should precede unrated books, including a book rated zero. Close/reopen and verify the review and sort choice persist.
9. Try **Clear rating**, then save with review text retained. Try **Remove review**: cancel first (keep the review), then confirm (remove rating and text). These actions should have no effect on Audible/mobile ratings or notes.
10. From either player, press the saved-moments button: the main app should show **Bookmarks & notes**, even if another section was selected. Save a note, navigate away and return; ensure existing annotation sync controls still behave as documented.
11. Sign out: library, saved moments, reviews and player must disappear. If you have another account, sign in and confirm it cannot see the first account's reviews. Sign back into the original account to verify its encrypted local reviews return.

If anything fails, report the app version, the step and visible error text. Do not include passwords, account details, signed-in URLs, private review text or a database file. Diagnostics do not contain review content.
