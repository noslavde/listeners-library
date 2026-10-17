# Listener’s Library: protected local storage

## Current behavior: 0.2.0

The app is now Listener’s Library; its internal application identifier, Windows encryption contexts and data directory stay unchanged so an update can still unlock existing credentials and files. The app remains independent and unaffiliated with Amazon or Audible.

Library metadata, notes, listening history, positions and current-book selections now use separate SQLCipher databases. New account registrations use a SHA-256 namespace derived from the Amazon customer identity in the authenticated registration response. That identity stays inside the sealed native account record. If the provider omits it, or an older credential lacks it, a separate namespace derived from that registration's refresh credential is used instead; this does not merge separate sign-ins. Different accounts never deliberately share an account database. Encryption keys, OS protection contexts and the working registration protocol are unchanged. General appearance/player preferences and website-retention choices remain local preferences shared across account connections.

**Upgrade limitation:** old `library.sqlite` data was never reliably bound to an account. It remains encrypted and preserved, but is not automatically copied into any account database. The active library syncs again. Previously saved notes, local positions and listening history in that unassigned file will not appear in the new profile. Do not run an older version to access it; a future deliberate recovery flow would need to establish ownership. This avoids exposing one person's retained data to the next person signing in. Missing provider identity can also prevent continuity across separate registrations; no match is guessed from book ownership or email.

Sign-out writes and flushes a nonsensitive `signed-out` marker before awaiting credential/browser cleanup. Startup refuses to restore a sealed account while the marker exists, even if deletion previously failed. A new sign-in retries website cleanup and old credential removal first; the marker is removed only after saving and decrypting the fresh account successfully. Cleanup errors are visible and must not be interpreted as a fully completed sign-out. The obsolete first-build recovery command is no longer available in the app.

Website cleanup destroys the live official page, uses a blank isolated cleanup view, waits for WebView2's full browsing-data deletion callback and verifies cookie removal before closing the view. It clears runtime-managed site storage as well as cookies; it cannot erase backups or guarantee forensic erasure. A timeout/failure keeps the signed-out marker. No Amazon credentials or real website profiles are used by automated tests.

Sections below describe the original protections and historical migration behavior; the 0.2.0 behavior above supersedes the former shared-account cache and recovery UI.

The library, notes, listening history, saved positions, launch hints and preferences use SQLCipher with its default authenticated encrypted-page format and a random 256-bit key. SQLCipher's crypto provider is statically bundled OpenSSL. There is no plaintext fallback. Temporary SQLite data uses memory, and normal journals are encrypted by SQLCipher. The database key is protected with Windows DPAPI in current-user scope, without CRYPTPROTECT_LOCAL_MACHINE. The key is never embedded in the executable or returned to React.

API access and refresh tokens are stored as a DPAPI-protected account file in the private data directory. This replaces the old shared Credential Manager entry, which is not imported. Protected account writes use a temporary encrypted file followed by replacement; there is no temporary plaintext account file. Native buffers holding the key, token serialization and decrypted DPAPI output are cleared when released where the app controls their allocation. This is not a promise that dependency-managed HTTP buffers or process memory contain no plaintext.

Development and installed builds use separate directories (`development-v2` and `release-v2`) and separate DPAPI application contexts. Their accounts, database keys and website profiles are separate. Normal updates within the same scope retain sign-in. Each profile permits one app instance at a time. The directory has a protected DACL granting full access to its owner and SYSTEM; child data files inherit it. This restricts other ordinary users, not administrators who can take ownership.

## Website sign-in

Audible's official player still needs WebView2 browser data. WebView2 owns this profile; SQLCipher does not encrypt it. The provider windows disable browser password saving and general form autofill. Provider pages receive no Tauri capabilities, and app credentials are never returned to them through IPC.

Under **Privacy & local storage**, turn off **Remember website sign-in on this PC** to use WebView2 private browsing for subsequent account/player windows. Changing this setting stops playback, closes sign-in windows and clears the current profile's cookies before changing modes. Private browsing avoids normal persisted website sign-in across sessions and is not forensic erasure of all browser/runtime traces. The encrypted API account remains remembered; use Sign out to remove it as well. Provider MFA and account verification may still be required.

## Upgrade and removal

Installed 0.1.1 migrates the old shared `library.sqlite` into an encrypted database on first use. It exports under a SQLite exclusive transaction, checks integrity and table row counts, verifies the new database can be unlocked, and only then publishes it and removes the old plaintext database. A Windows-protected digest receipt allows cleanup to resume after interruption. If the legacy source changes or verification fails, startup stops with both recoverable copies retained. Close all older app versions before upgrading. Do not run the older build afterward: it can create another unencrypted store.

Development builds start a separate empty store and do not import the old shared library. Existing account credentials and website sessions are not migrated; sign in again once in the new build. The old browser profile and obsolete Credential Manager entry remain until **Remove old version data** is confirmed. This preserves a deliberate recovery path without silently reusing that account.

**Sign out** removes the current protected API account and website cookies, retaining the encrypted library and notes. **Remove old version data** permanently removes known legacy shared files/profile and its obsolete account credential, retaining the current encrypted library. **Erase local app data** removes the current scope's encrypted data, key, account and website profile; installed builds also remove those known legacy stores. Removal requires a typed confirmation and restart. Cleanup runs before accounts or databases reopen, checks for linked paths and stops on failure. A nonsensitive tombstone prevents erased data being imported again. Other devices and the Audible account itself are unaffected.

Deleting data does not erase existing backups, cloud filesystem copies, SSD recovery remnants, pagefiles or past exports. Protect the Windows device and backups separately. Missing/corrupted encryption keys are not silently regenerated over an existing database, and there is no developer recovery key. Losing access to the Windows-protected key can make notes permanently inaccessible.

## Threat boundary and validation

These protections reduce exposure from accidental copying of local files and access by other ordinary Windows users. DPAPI is a Windows-user security boundary, not an app-exclusive vault. Malware running as the same user, an administrator, debugger, compromised dependency, or compromised operating system can still access decrypted data or invoke Windows unprotection. DPAPI behavior can also depend on Windows account/profile backup and recovery. Files can still be copied even when their contents are encrypted. No claim of absolute nontransferability or an independent security audit is made.

Window geometry and the cached appearance choice are nonsensitive browser/plugin data outside SQLCipher. Audible/Amazon receive normal authenticated API/playback traffic, and official pages may collect their own website telemetry. The app adds no analytics, crash uploader, remote notes service or credential export. Installer resources remain allowlisted to app assets and dependency license notices.

Regression tests use synthetic data for Windows protection/tamper rejection, encrypted database pages, wrong keys, missing/corrupted keys, migration preservation and bounded removal. Account sign-in, private browser behavior, migration on the user's existing installation, and clean-machine installation still require live testing. Never provide credentials, cookies, MFA codes or signed-in URLs in bug reports.

## Historical default-browser account connection (0.1.5 only)

Account connection now opens the Windows default browser. Password/MFA entry, browser cookies and browser history are managed by that browser. The app does not read or import its cookie/profile files. The user pastes the final Amazon HTTPS return address into a masked, non-persisted app field. It contains a temporary authorization code; it must not be shared in bug reports. The field clears before submission. The app validates its origin/path, consumes it once with the in-memory PKCE attempt, and sends the code only to the fixed Amazon registration endpoint. The attempt expires after ten minutes or can be canceled. The app does not log/store the pasted address; Windows clipboard/history and browser history are outside the app’s control.

The resulting tokens use the existing Windows-protected storage. Default-browser cookies are separate from the official player WebView. The app’s remember-website setting and Sign out do not clear the external browser’s own cookies or sign out that browser.

## Explicit first-build account recovery (0.1.6)

Default-browser account sign-in has been removed from the active UI and native command registration. Sign in uses the first installer’s Android WebView flow, subject to the user’s website privacy preference. **Account → Restore account from first build** is an optional, installed-build-only action that reads the original Windows Credential Manager entry `AudibleDesktop.Metadata.US` for `default` inside the native app and re-seals its tokens into current Windows-protected account storage. It verifies that the new record can be decrypted before reporting a connection. There is no automatic import, network credential export or UI token response. The old entry is left unchanged; Remove old version data removes it if the user requests cleanup. A fresh installation without this user’s saved credential cannot restore that account. Existing encrypted library files and keys are preserved.

## Sign-out access boundary (0.1.10)

Sign out now immediately revokes native account access and clears the app’s displayed library, series, active title, current-book selection and notes. It stops playback, cancels timers/launch handoff, closes the mini player, clears Windows media metadata, clears website cookies and destroys the official WebView. Account tokens are removed; installed builds also remove the obsolete Credential Manager entry so old-account recovery cannot undo intentional sign-out. Known cleanup failures remain visible and do not reopen in-process access.

Encrypted notes and retained caches remain on disk, but cached-data commands return empty results while disconnected, including after restart. Playback and media shortcuts require a connected account. Sign-out does not delete purchases or sign out other devices. To permanently remove retained local files, use Erase local app data. This is disconnected-state protection, not a multi-account database partition system.
