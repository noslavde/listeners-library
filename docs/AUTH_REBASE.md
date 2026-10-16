# First installer authentication rollback with encrypted storage — 0.1.6

The user confirmed that the first installer opened already connected. That proves reuse of a locally saved account, not a successful new authorization in the packaged application. This build offers explicit recovery of that account and restores the first installer's sign-in selection for new authorization.

## Correct source provenance

The exact first NSIS-enabled 0.1.0 project snapshot was recovered from local Git tree `e21c0ea492beea3cef2d58cc85ec378d11f456dc`. Its metadata source, profile, UI, configuration and main file are preserved under ignored `work/auth-rebase/packaged-first-build/`. No live credentials or user data were copied. The repository has an unborn branch, so no reset/rebase or commit-history rewrite was performed.

The previous 0.1.3 rollback was incorrectly based on earlier metadata prototype tree `80ac9fcd41785d3a588457b4f769596199ec0e44`. That prototype used iOS; the actual packaged 0.1.0 UI explicitly selected Android, and its native login default was Android. The registration profile implementation is identical to the actual packaged checkpoint.

## Restored behavior

- Sign in opens the official Amazon forms in the app's shared account/player WebView and requests the original packaged Android registration profile.
- The first installer's authorization URL construction, registration request body/headers and profile-dependent refresh behavior are retained.
- The 0.1.5 external-browser return-address UI, commands and active module are removed; source is backed up under ignored `work/auth-rebase/pre-0.1.6/`.
- Existing library/player features are retained. This is a targeted authentication rollback, not a whole-project reset.

## Explicit existing-account recovery

**Account → Restore account from first build** reads `AudibleDesktop.Metadata.US` / `default` from the current user's Windows Credential Manager inside the native app. The command is guarded to the local main window and available only in an installed build. It does not run at startup or import accounts automatically.

The record's serialized API profile and refresh token are preserved; older records without a profile retain their original iOS default. Expired access tokens are allowed because the normal Sync library path refreshes them. The record must parse and contain bounded nonempty token fields. The native app writes current DPAPI-protected account storage and verifies decryption before setting connected state. Nothing credential-bearing is returned to the UI. The original credential remains protected in Credential Manager until the user requests legacy cleanup. Missing/inaccessible/invalid records give fixed, non-secret error text and are left unchanged.

Restoration alone does not prove Amazon still accepts the old refresh token. Choose Sync library to verify actual account access. If the entry was removed by an earlier reset, it cannot be reconstructed by rolling back source.

## Security retained

SQLCipher library/notes/history/settings encryption, the random Windows-protected database key, DPAPI account-token encryption, directory permissions, separate release/development stores and identities, safe migration, secure-save read-back, website cookie handling and confirmed data removal remain. Storage scope and encryption contexts do not change with this version. The app does not regenerate keys or clear user data for the rollback.

Callback origin validation, single-code parsing, the post-callback close-race fix, durable connection status and bounded numeric token lifetimes remain. The optional website-private-session preference and disabling browser password saving/autofill remain. These are documented differences from 0.1.0; they do not modify the account registration payload. There is no plaintext or memory-only fallback when secure storage fails.

## Validation and limits

Synthetic tests cover previous records with and without serialized profiles, expired-but-refreshable access tokens, malformed records without echoing input, protected save/decrypt/read-back and public status without credentials. A separate regression pins the actual packaged Android authorization/registration/refresh constants. Existing synthetic HTTP registration, encrypted persistence, cancellation, callback and privacy checks remain.

Tests do not read the user's live Credential Manager entry. Actual restore, refresh/library sync and fresh provider sign-in still need live testing. Do not share tokens, account files, browser console output, sign-in URLs, passwords or MFA codes.

## Confirmed registration rejection and correction — 0.1.8

The user's live 0.1.7 result identifies `DuplicateDeviceName` at HTTP 400 during account exchange. The first build and subsequent versions sent a fixed `Audible Desktop Lab` registration name. This was retained by the source rollback and can collide with an account's existing registration. Encryption/save has not run when this error occurs.

New registration names use `Audible Desktop` followed by a 128-bit hexadecimal suffix derived from the attempt's existing random serial. The same attempt keeps the same name, different serials produce distinct names, and no account/hostname/user data is included. Both iOS and Android profiles use the correction. This is an intentional change from the packaged source: the remaining registration contract and encrypted storage are retained. No devices are deregistered, no account data is reset, and the original authorization code is not retried automatically. A fresh sign-in is needed after installation. The specific rejection is confirmed; successful live sign-in after the fix is still unverified.
