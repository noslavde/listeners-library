# Windows close, reopen and diagnostics checks

1. If the old version is already stuck, open Task Manager and end its **listeners-library.exe** process. Install the new Setup.exe. Keep existing data; do not delete keys or use Erase as troubleshooting.
2. Open the library, close it with X, then check that the app process exits. Reopen from Start. The library should remain visible. Repeat while disconnected, connected, paused and playing (note your position before closing).
3. Open the mini player. It should hide both library and official player. Back to app or Alt+F4 should restore the library and keep playback. Closing the library afterward should stop audio and exit, with no hidden app process.
4. Launch the Start shortcut while the app is in mini mode, hidden or minimized. The existing library should appear; there should still be just one main app process. Development builds use a separate instance scope.
5. Reopen and play the same book. Verify progress recovery still works. Switch books once; the correct title should launch. Sign out and verify playback and account UI are cleared.
6. Open **Troubleshooting & diagnostic report → Review diagnostic report**. Inspect every field. Save and inspect the JSON file. It should exactly match the preview. Cancel the save dialog once; no report should be created. Nothing uploads automatically; explicitly attach the JSON only if you want to share it.
7. Close normally and reopen; a newly reviewed report should have `previous_session_ended_uncleanly: false`. Then, with playback paused and notes saved, end this new version's process in Task Manager and reopen. The field should be true, without claiming a proven crash. That changes back after a subsequent normal close/reopen.
8. If a problem occurs, save a report before restarting repeatedly: only two sessions and 128 events per session are retained. Describe the actions leading to the problem without account details. Avoid full browser logs, account files and memory dumps.

Automated tests cover Windows instance signalling, diagnostic data validation, bounds, termination classification, privacy of frontend error forwarding, and existing storage/playback logic. They do not establish actual GUI activation or the behavior of every WebView2/Windows policy. Real close/reopen and installer tests remain necessary.

If the installed app cannot open at all, enter `%LOCALAPPDATA%\dev.audible.desktop.lab\diagnostics-release-v2` into File Explorer. If present, `events.json` is the same limited technical journal without the export's version wrapper. Review it before sharing. Do not attach neighboring account/database/browser files. Very early startup failures or unavailable disk storage may leave no journal.
