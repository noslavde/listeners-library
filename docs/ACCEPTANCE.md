# Account acceptance checklist

Use your own account on an approved Windows development machine. Do not share passwords or export cookies.

1. Run Rust checks/tests and `npm run tauri dev`; verify official pages cannot invoke app commands.
2. Sign in including MFA; verify restart behavior and record whether the official profile retains login. Closing the official window should make custom controls unavailable after the 2.5-second telemetry timeout.
3. Import a library export; restart and confirm cached metadata/notes persist. Verify authors, series and durations against a known title.
4. Play an owned title using the official player. Confirm audible audio, correct duration/position and available custom controls. Check play rejection/autoplay and inaccessible media cases.
5. Test ±15 seconds, direct seek and playback rates against official position. Browse a series while playing; no pause, reload or position reset should occur.
6. Verify chapter selector before claiming pause-after-chapter works. Confirm pause at transition and compare chapter percentages to supplied chapter offsets.
7. Pause for over 60 seconds, resume and verify selected rewind. Cause or observe a real unintended jump, then verify recovery target. Ordinary commanded seeks must not overwrite recovery.
8. Save a timestamped note, restart, select its title and seek from the note.
9. Verify remote-page navigation outside the explicit HTTPS host allowlist is denied and unsupported media leaves controls disabled.

Record marketplace, WebView2 version, date, ASIN (optional/private), results and failure reasons. No account-dependent item has been marked passed during scaffolding.
