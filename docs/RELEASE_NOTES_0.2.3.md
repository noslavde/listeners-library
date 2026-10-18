# Listener’s Library 0.2.3 beta — progress protection and book switching

- Fixed a checkpoint overwrite: a newly opened or paused official player no longer replaces the PC's last confirmed listening position. Checkpoints save every five seconds during listening and flush on pause, title switch and normal app close.
- If the opened position differs from the PC checkpoint by more than 15 seconds, playback stays paused and **History** offers both. The PC checkpoint is dated. The integration has no verified cloud listening time, so the app does not silently declare the cloud or furthest position newer.
- **History** provides a dated, encrypted recovery log. It stores metadata only: up to 100 entries per book, 3,000 per account and 90 days of recent entries, pruned on saves. The latest checkpoint per book is retained separately. Existing records from older builds are preserved where available; previously overwritten progress cannot be reconstructed.
- Book changes now close the previous playback view and open a new one using the same website profile. Old view callbacks cannot redirect the new player, and launch operations are serialized. The sign-in implementation and encrypted account store are unchanged.
- **Previous** restarts the current chapter. Another click within three seconds steps to the previous chapter; quick further clicks continue backward. Other navigation clears the repeat state. The dropdown remains available.

Local Windows test build. Verify real title switching, resume after closing/updating and mobile handoffs before publication. See [test checks](MINI_PLAYER_TEST.md) and [getting started](TESTER_GUIDE.md).
