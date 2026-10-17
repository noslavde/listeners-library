# Listener’s Library 0.2.1 beta

Download **Listeners-Library-0.2.1-Windows-x64-Setup.exe** under Assets. Source code ZIP/TAR files are not installers. Windows 10/11 x64, Audible US, your own account and internet are required. The installer is unsigned; do not disable Windows protections if your device blocks it.

This beta improves data handling and recovery while preserving the working Amazon sign-in flow:

- Notes stay with their book and initial typing timestamp. Read without playback, edit/delete saved notes and explicitly export a readable JSON backup.
- Old unassigned notes can be deliberately exported without guessing ownership or merging accounts.
- Conflicting desktop/account positions are shown honestly instead of favoring a stale local checkpoint.
- Added cancel-sign-in, retry-sign-out, clearer playback fallback and a UI recovery screen.
- Updated the locked TLS dependency to resolve four RustSec findings. Current npm and cargo audits report zero known vulnerabilities; non-Windows upstream informational notices remain documented.
- Added ongoing dependency/format/build gates, MIT source license and plain-language installation/troubleshooting/privacy guides.

**Upgrade:** close all older copies first. Listener’s Library 0.2.0 account notes stay in place. Older shared notes/history remain encrypted and are not automatically assigned; use Notes & backups for explicit old-note export. Do not run the previous Audible Desktop app to bypass account isolation.

**Limits:** not independently security-audited; clean-machine, real account-switching/private-mode and interrupted-cleanup tests are still required before a stable release. Provider verification/manual Listen now may still be necessary. Missing provider identity may create a different local profile on a later sign-in; export important notes before signing out. Exported JSON is unencrypted and must be kept private. No note import, offline audio, updater or code signing is included.

Read TESTER_GUIDE.md and PRIVACY.md before testing. Report visible messages and steps; never share passwords, MFA codes, cookies, signed-in URLs, keys or account files.
