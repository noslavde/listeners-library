# Experimental macOS alpha — untested

This is a separate Mac side release. The Windows beta is unchanged. **Real Amazon sign-in, full-book audio, mini-player behavior and sign-out have not been tested by a Mac user.**

## Install

Requires macOS **14 Sonoma or newer** and an Audible US account with access to the books you play.

- **M-series Mac:** download the **Apple-Silicon-EXPERIMENTAL.dmg**.
- **Intel Mac:** download the **Intel-EXPERIMENTAL.dmg**.

Open the DMG and drag **Listeners Library Experimental** into Applications. Source code ZIP/TAR files are for developers. SHA256SUMS.txt contains the download checksums.

The app is ad-hoc signed, **not Developer ID signed or notarized**. macOS may block it. Do not disable Gatekeeper or remove quarantine protection to test it; stop and report the result.

## What to expect

The library, series, notes, themes, volume, sleep timer and mini-player UI are included. Audio stays in the official website player. **Mac global shortcuts and headset/system media keys are not implemented.** Audible may require additional verification or may not support playback in the embedded WebKit runtime.

Local library/notes use SQLCipher. Keychain protects the account and database secrets using authenticated encryption. The app has a separate website store; it does not use Safari's sign-in. No account data or signing credentials are packaged. This is not an independently audited product.

Read **READ-ME-FIRST.md** under Assets or the [Mac installation, privacy and testing guide](https://github.com/noslavde/listeners-library/blob/codex/macos-experimental/docs/MACOS_EXPERIMENTAL.md) before using an account. The first useful checks are fresh sign-in, full-book audio/chapters, mini-player hide/restore and sign-out during playback. Share visible errors only, never account details, sign-in URLs, cookies, databases or Keychain exports.

Automated builds run frontend/native tests and dependency audits, then verify the completed disk images and embedded app signatures. These checks do not establish that real account sign-in or audio playback works. Keep this alpha separate from the recommended Windows release.
