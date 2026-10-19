# Listener’s Library 0.2.4 beta — reliable shutdown and private diagnostics

- Closing the main library explicitly exits the application, including hidden playback windows. Confirmed progress and window placement save first. There is no background/tray mode. Closing the mini player still returns to the library and keeps playback running.
- Launching the app again restores the existing library window instead of starting another copy that competes for protected storage. Development and installed builds remain separate.
- Startup explicitly shows and restores the library. Opening the mini player restores it before hiding the library; losing the mini window restores the library. Pending automatic playback transitions are cancelled when returning through a second launch or closing the app.
- Added **Troubleshooting & diagnostic report**. Review and save a bounded local report; nothing is uploaded automatically. It contains app version, architecture, relative event times and predefined lifecycle/error codes. No accounts, books, notes, URLs, file paths, raw error messages, stack traces or memory dumps are included.
- Rust panics and frontend failures produce limited diagnostic codes where possible. An incomplete previous session is marked **unclean**, which can also mean a forced stop or power loss. This is not comprehensive Windows crash capture. Reports are deliberately sparse to protect private information.

Local Windows test build; not published. If an older build has already left an invisible process, end **listeners-library.exe** in Task Manager before installing this update. The old executable cannot receive the new activation signal. Do not erase account data or delete encryption keys.

Before publication, verify closing/reopening, launching twice, mini-player return, progress retention, installer upgrade/uninstall, and report preview/export on a real Windows installation. See [lifecycle test checks](LIFECYCLE_TEST.md).
