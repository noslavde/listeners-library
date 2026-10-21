# Source module index

Use this index to locate responsibilities. Module headers and nearby contract comments
are the primary explanation; update this index when moving or changing modules.
Generated output and dependency code are excluded. For data flow and security
boundaries, start with [the developer walkthrough](DEVELOPER_GUIDE.md).

## scripts

- [build-notices.mjs](../scripts/build-notices.mjs): Generates bundled
  third-party notices from dependency metadata and available license texts.
  Output is generated, not hand-maintained; source protocol attributions are
  included alongside dependency licenses.
- [check-release-workflow.mjs](../scripts/check-release-workflow.mjs): Synthetic
  release-workflow regression harness using a stub GitHub CLI. Exercises
  draft/published asset handling and checksum policy without publishing a real
  release.
- [check-release.mjs](../scripts/check-release.mjs): Release packaging
  assertions for the built frontend and native command registration. Rejects
  development/private resources and requires experimental native commands to
  remain immediately cfg-gated, independent of formatting whitespace.
- [ci-perl.ps1](../scripts/ci-perl.ps1): CI setup for the pinned Windows Perl
  distribution used by bundled OpenSSL. Verifies the downloaded archive before
  extraction and exposes the selected executable to later native build steps.
- [native-build.mjs](../scripts/native-build.mjs): Windows native command
  wrapper. Selects the verified local Perl toolchain when available and forwards
  to Cargo tests or Tauri dev/NSIS builds without embedding account
  configuration.

## src-tauri

- [build.rs](../src-tauri/build.rs): Standard Tauri build hook for generated
  platform resources and configuration. Keep account/storage behavior in runtime
  modules rather than build scripts.

## src-tauri/src

- [activity.rs](../src-tauri/src/activity.rs): Meaningful-listening credit for
  the Continue shelf. Samples natural playback progression, rejects seek jumps
  and pins pending credit to the captured account generation before persistence.
- [annotation_sync.rs](../src-tauri/src/annotation_sync.rs): Explicit
  experimental annotation reconciliation. Stable remote IDs, a three-way
  baseline and durable pending-write intents preserve local/cloud conflicts and
  prevent blind retries after uncertain sends; no automatic cloud deletion is
  performed.
- [bridge.js](../src-tauri/src/bridge.js): Injected official-player
  control/telemetry bridge for allowlisted Audible pages. Reads media state and
  enforces resume/clip/chapter guards; it has no general Tauri command access
  and does not handle account secrets.
- [cover.rs](../src-tauri/src/cover.rs): Public cover-artwork URL validation and
  separate artwork-window lifecycle. Checks local callers and the current
  selected book; artwork viewing must not change playback or expose signed
  website URLs.
- [current_book.rs](../src-tauri/src/current_book.rs): Explicit per-account
  current-book pin. This selection survives playing a different title and stays
  separate from activity/history-derived Continue listening eligibility.
- [desktop.rs](../src-tauri/src/desktop.rs): Shared native player state and
  main/mini window commands. PlayerHub revisions invalidate stale control work;
  fresh official-player observations are distinct from persisted listening
  checkpoints.
- [diagnostics.rs](../src-tauri/src/diagnostics.rs): Bounded privacy-filtered
  diagnostic journal and user-reviewed export. Records allowlisted codes and
  relative timing only; unclean termination is a clue, not proof of a crash, and
  no report is automatically uploaded.
- [highlights.rs](../src-tauri/src/highlights.rs): Encrypted bookmark/note/clip
  metadata persistence and confirmed replay actions. Validates ranges against
  cached books, checks expected values before edits and uses the official
  player's timeline rather than recording audio.
- [history.rs](../src-tauri/src/history.rs): Encrypted listening checkpoints,
  bounded history and startup conflict protection. Saves confirmed
  listening/explicit choices, holds startup when positions disagree and never
  treats metadata-read time as a cloud listening timestamp.
- [launch.js](../src-tauri/src/launch.js): Injected title-selection helper for
  the signed-in official library. Finds the requested book's own Listen now
  control, searches/highlights when necessary and reports bounded launch
  progress back to the host.
- [launch.rs](../src-tauri/src/launch.rs): Account-scoped official-player launch
  history and callback-instance validation. Remembers only cleaned full-player
  URLs for a selected book; stale windows must not launch or update a later
  selection.
- [library.js](../src-tauri/src/library.js): Injected library-page metadata
  reader used by the development import path. Extracts visible book metadata
  into a host callback rather than reading sign-in fields or treating the page
  as an unrestricted IPC client.
- [library_ready.js](../src-tauri/src/library_ready.js): Minimal injected
  library-readiness signal. Reports DOM readiness before all page resources
  load, while rejecting unrelated origins and account-sign-in pages.
- [lifecycle.rs](../src-tauri/src/lifecycle.rs): Windows single-instance
  activation, hidden-window recovery and orderly shutdown. Closing the main app
  must end playback/process ownership rather than leave an inaccessible
  background instance.
- [listening.rs](../src-tauri/src/listening.rs): Validated preferences, app
  volume and sleep timers. Preferences use the shared encrypted settings store;
  live observations enforce time/chapter boundaries and report timer completion
  to local windows.
- [main.rs](../src-tauri/src/main.rs): Windows native host and Tauri command
  composition root. Registers state, validates local/remote origins, launches
  official website windows and wires lifecycle cleanup. Development-only
  commands must remain cfg-gated; feature logic lives in sibling modules.
- [media_keys.rs](../src-tauri/src/media_keys.rs): Windows media-session
  integration for hardware/headset controls. Routes supported actions into the
  same native shortcut path and updates display state from player observations.
- [metadata.rs](../src-tauri/src/metadata.rs): Native account session and
  metadata coordinator. Credentials stay in native protected storage; account
  generations and the durability guard prevent stale requests from writing into
  another account. Library API registration does not guarantee website or DRM
  playback authentication.

## src-tauri/src/metadata

- [account_restore.rs](../src-tauri/src/metadata/account_restore.rs): Test-only
  compatibility parser for the retired first-build account-restore format. It is
  not exposed as a release recovery command; existing fixtures must never be
  replaced with real credentials.
- [annotations.rs](../src-tauri/src/metadata/annotations.rs): Experimental
  signed annotation-service transport. Native-only signing material is optional
  at registration, request/response sizes are bounded, redirects are rejected
  and account-generation checks surround network work.
- [browser_test.rs](../src-tauri/src/metadata/browser_test.rs): Development-only
  local HTTP comparison host for ordinary-browser protected playback. Pairing
  and same-origin checks constrain access; this is an experiment, not a public
  credential-sharing endpoint.
- [login_tests.rs](../src-tauri/src/metadata/login_tests.rs): Synthetic
  regression fixtures for account registration and callback parsing. Tests
  protect request/profile behavior without signing into Amazon or storing a real
  account.
- [profile.rs](../src-tauri/src/metadata/profile.rs): Account-registration
  profile constants and request construction. API device identity is separate
  from the browser's DRM identity; changing a profile does not establish trusted
  protected playback.
- [stream.rs](../src-tauri/src/metadata/stream.rs): Development-only
  protected-stream grant, fetch and license-relay experiment. Keeps native
  session context and bounded network responses; a granted entitlement is not
  evidence that the platform license challenge will be accepted.
- [upload.rs](../src-tauri/src/metadata/upload.rs): Development-only
  progress-upload experiment with explicit prepare/commit steps. Separates
  inspection from a deliberate service write and is not the shipped official
  player's ordinary progress-sync path.
- [website.rs](../src-tauri/src/metadata/website.rs): Native preparation of the
  separate Audible website session from account context. Cookies remain
  native/browser-side; website verification may still be required even when
  library API authentication succeeds.

## src-tauri/src

- [notes.rs](../src-tauri/src/notes.rs): Account-pinned saved-note reads,
  compatibility commands and reviewed local exports. Exports deliberately
  produce readable private content and require explicit confirmation;
  diagnostics never include these note bodies.
- [player_sync.rs](../src-tauri/src/player_sync.rs): Explicit Audible-position
  fetch and confirmed seeking. Pins account, book and player revision across
  async work; manual controls or a session change cancel the pending result
  rather than apply it to a new player.
- [privacy.rs](../src-tauri/src/privacy.rs): Website-session preference and
  explicit local-store erasure workflow. Stops/revokes active account work
  before cleanup; UI confirmation tokens and origin checks guard destructive
  operations.
- [protection.rs](../src-tauri/src/protection.rs): Windows DPAPI and
  owner/SYSTEM file-permission helpers. Development/release entropy differs;
  these protections reduce accidental exposure but do not isolate secrets from a
  compromised Windows user or administrator.
- [reviews.rs](../src-tauri/src/reviews.rs): Private per-account rating/review
  persistence with no service calls. Ratings use integer tenths, optimistic
  expected-value checks protect edits, and all commands require the trusted main
  window and captured account generation.
- [shortcuts.rs](../src-tauri/src/shortcuts.rs): Native app/global shortcut
  registration and action dispatch. Commands use current player/account checks
  and publish fixed notices instead of persisting typed user input.
- [start_playback.rs](../src-tauri/src/start_playback.rs): Pending automatic
  transition from official playback to the mini player. Requires matching book,
  ready full playback and a current request; cancellation prevents late
  callbacks from hiding the app unexpectedly.
- [storage.rs](../src-tauri/src/storage.rs): Encrypted local storage and
  verified legacy migration. Per-scope SQLCipher keys are randomly generated and
  Windows-protected; profile databases are account-isolated, writes are
  bounded/atomic where required, and missing keys never trigger plaintext
  fallback.
- [volume.js](../src-tauri/src/volume.js): Injected app-only media volume
  controller for official playback. Uses normal media attenuation (0..100%)
  without creating a Web Audio graph; explicit volume choices alone may unmute
  the media.
- [window_state.rs](../src-tauri/src/window_state.rs): Window
  size/position/maximization persistence and close-event routing. Visibility is
  intentionally not restored; debounced placement checkpoints survive
  development restarts without recreating a hidden app.

## src

- [App.tsx](../src/App.tsx): Main-window orchestration for account state,
  library navigation and the persistent player. Account epochs and request
  revisions discard stale async work; hidden sections stay mounted to preserve
  drafts without recreating playback.
- [BrowserTestControls.tsx](../src/BrowserTestControls.tsx): Development-only
  controls for starting/stopping an ordinary-browser playback comparison.
  Displays sanitized session status; excluded from the shipped main window.
- [CardProgress.tsx](../src/CardProgress.tsx): Presents a book's observed, local
  and cloud positions using progressSummary. Conflicting positions remain
  visible rather than selecting the furthest position as a guess.
- [ChapterSelect.test.tsx](../src/ChapterSelect.test.tsx): Regression tests
  covering chapter availability. Uses synthetic fixtures; these checks do not
  verify real account sign-in, audio output or mobile synchronization.
- [ChapterSelect.tsx](../src/ChapterSelect.tsx): Chapter dropdown driven by
  cached book metadata and live seekability. Delegates chapter selection or
  explicit seeking to its caller instead of assuming metadata alone proves
  playable content.
- [Cover.tsx](../src/Cover.tsx): Reusable cover image with a title-initial
  fallback. Only allowlisted public Amazon image URLs are rendered; missing or
  invalid artwork does not block library use.
- [CoverArtwork.tsx](../src/CoverArtwork.tsx): Dedicated cover-artwork window.
  Reads the native-selected safe image and closes independently of playback; no
  signed-in website or account token is rendered here.
- [CurrentBook.tsx](../src/CurrentBook.tsx): Explicit current-book selection and
  its library card. Selection is distinct from the title currently playing;
  account-change events clear state before reloading the appropriate store.
- [DiagnosticsPanel.tsx](../src/DiagnosticsPanel.tsx): Review-before-export UI
  for the bounded diagnostic report. Shows exactly what will be saved locally,
  and never uploads reports automatically.
- [DirectPlaybackPanel.tsx](../src/DirectPlaybackPanel.tsx): Development-only
  direct protected-streaming experiment UI. Exercises runtime/grant readiness
  without replacing the shipped official-player route or bypassing DRM.
- [ErrorBoundary.tsx](../src/ErrorBoundary.tsx): Last-resort React render
  failure screen. Reports only a fixed error code and offers recovery without
  resetting saved data or exposing exception details.
- [ExploreNav.tsx](../src/ExploreNav.tsx): Library-series navigation with an
  accessible selected state. The series list describes owned/cached books, not a
  separate Audible store catalog.
- [HighlightsPanel.tsx](../src/HighlightsPanel.tsx): Bookmarks, clip ranges and
  timestamped notes UI, including explicit experimental Audible annotation sync.
  Keeps drafts per book, pins requests to an account generation, and preserves
  separate local/cloud versions on conflict.
- [LibraryShelf.tsx](../src/LibraryShelf.tsx): Continue-listening shelf and
  native activity subscription. Meaningful listening, expiration and dismissal
  rules live in libraryView; opening or briefly testing a title is not enough to
  qualify.
- [ListeningHistory.tsx](../src/ListeningHistory.tsx): Account/book-scoped
  listening history and recovery UI. Recovery is an explicit choice between
  local and Audible positions, not a claim that either service exposes the
  newest listening timestamp.
- [MetadataPanel.tsx](../src/MetadataPanel.tsx): Account connection and
  library-sync UI. Passwords are entered on official Amazon pages; this
  component receives status and invokes native commands rather than handling
  credentials.
- [MiniPlayer.tsx](../src/MiniPlayer.tsx): Shared compact playback controls for
  the mini window and embedded main player. Embedded mode suppresses mini-only
  dragging, resizing and duplicate keyboard listeners; playback and persistence
  remain native responsibilities.
- [NotesPanel.tsx](../src/NotesPanel.tsx): Compatibility entry point for the
  saved-moments feature. Re-exports HighlightsPanel so existing callers use the
  richer bookmarks/notes UI without a second implementation.
- [PrivacyPanel.tsx](../src/PrivacyPanel.tsx): Local privacy controls for
  website-session persistence and deliberate data erasure. Uses native
  review/confirmation flows; it does not access credentials or storage keys
  directly.
- [ResumePanel.tsx](../src/ResumePanel.tsx): Manual saved-position and chapter
  refresh controls for an active book. Request generations keep responses for a
  previous book from replacing the current book's metadata.
- [ThemePicker.tsx](../src/ThemePicker.tsx): Theme synchronization and
  accessible palette/mode selection. Applies tokens through themes.ts and saves
  choices through the listening-preferences hook.
- [VolumeControl.tsx](../src/VolumeControl.tsx): App-only volume slider with a
  responsive draft and debounced command updates. Values are normalized to 0..1,
  reset means 100%, and failed writes roll back the current draft.
- [accountSync.test.ts](../src/accountSync.test.ts): Regression tests covering
  automatic account library sync. Uses synthetic fixtures; these checks do not
  verify real account sign-in, audio output or mobile synchronization.
- [accountSync.ts](../src/accountSync.ts): Coordinates account-triggered library
  refreshes. Coalesces overlapping sync requests and invalidates stale results
  so an old account cannot repopulate the next account's UI.
- [annotationSync.css](../src/annotationSync.css): Explicit annotation-sync
  status and conflict-review styles. Badge classes are distinct from
  conflict-container classes to prevent unrelated layout rules from leaking into
  status labels.
- [bridge.test.ts](../src/bridge.test.ts): Regression tests covering official
  media seek boundary. Uses synthetic fixtures; these checks do not verify real
  account sign-in, audio output or mobile synchronization.
- [browserComparison.tsx](../src/browserComparison.tsx): Development-only
  ordinary-browser comparison entry point. Pairs to the local native experiment
  and uses the same protected-stream engine to distinguish runtime issues from
  host integration issues.
- [browserTransport.test.ts](../src/browserTransport.test.ts): Regression tests
  covering browser comparison transport. Uses synthetic fixtures; these checks
  do not verify real account sign-in, audio output or mobile synchronization.
- [browserTransport.ts](../src/browserTransport.ts): Development-only
  same-origin bridge to the local browser comparison server. Consumes a one-use
  pairing ticket from the fragment, removes it from the URL and rejects
  redirects/non-JSON responses.

## src/components

- [OfficialPlaybackControls.tsx](../src/components/OfficialPlaybackControls.tsx):
  Manual official-window visibility and website-session recovery actions.
  Library API connection and website sign-in are separate states, so resetting
  one does not silently imply resetting both.

## src

- [diagnostics.test.ts](../src/diagnostics.test.ts): Regression tests covering
  private error reporting. Uses synthetic fixtures; these checks do not verify
  real account sign-in, audio output or mobile synchronization.
- [diagnostics.ts](../src/diagnostics.ts): Frontend error reporting through
  fixed diagnostic codes. Raw exceptions, URLs, stack traces and user content
  are deliberately not forwarded to the diagnostic journal.
- [directCommands.test.ts](../src/directCommands.test.ts): Regression tests
  covering direct playback controls. Uses synthetic fixtures; these checks do
  not verify real account sign-in, audio output or mobile synchronization.
- [directCommands.ts](../src/directCommands.ts): Development-only in-process
  command handoff to the direct streaming experiment. Registration returns
  cleanup so a destroyed player cannot keep intercepting commands.
- [directPlayback.ts](../src/directPlayback.ts): Development-only
  protected-media capability probe. A supported browser key system proves
  neither Audible entitlement nor acceptance of this app's license request.

## src/features/reviews

- [Rating.tsx](../src/features/reviews/Rating.tsx): Accessible star display for
  integer-tenths ratings. Null/undefined means unrated; zero is a valid rating
  and must keep its numeric label.
- [ReviewsPage.tsx](../src/features/reviews/ReviewsPage.tsx): Private local
  rating/review editor and reviewed-book list. Drafts are keyed by book and
  carry the saved value they were based on; navigating sections does not discard
  them.
- [reviewModel.test.ts](../src/features/reviews/reviewModel.test.ts): Regression
  tests covering private ratings. Uses synthetic fixtures; these checks do not
  verify real account sign-in, audio output or mobile synchronization.
- [reviewModel.ts](../src/features/reviews/reviewModel.ts): Local review
  contract, validation and rating order. Store integer tenths (0..50), display
  0.0..5.0, and treat null as unrated rather than zero.
- [useReviews.ts](../src/features/reviews/useReviews.ts): Account-scoped local
  review loading and optimistic saves. The backend generation pins storage
  identity, UI revisions reject stale completions, and the browser demo keeps
  only in-memory values.

## src

- [highlights.css](../src/highlights.css): Saved-moments layout, composer,
  filter and passage-card styles. Uses shared theme variables;
  component-specific classes keep the notes interface separate from the
  persistent player.
- [launch.test.ts](../src/launch.test.ts): Regression tests covering early
  library readiness; official library launch. Uses synthetic fixtures; these
  checks do not verify real account sign-in, audio output or mobile
  synchronization.
- [libraryView.test.ts](../src/libraryView.test.ts): Regression tests covering
  Continue listening policy; library sorting. Uses synthetic fixtures; these
  checks do not verify real account sign-in, audio output or mobile
  synchronization.
- [libraryView.ts](../src/libraryView.ts): Pure sorting and continue-listening
  eligibility rules. Returns copied arrays for display and uses verified
  listening activity rather than cloud refresh time to rank recent listening.
- [listening.tsx](../src/listening.tsx): Native listening preferences,
  sleep-timer status and their controls. The hook subscribes to settings
  changes; the backend owns validation, persistence and actual timer/volume
  behavior.
- [main.tsx](../src/main.tsx): React entry point for main, mini-player and
  cover-artwork windows. Shared error reporting and theme initialization run
  before rendering; business state belongs in App and feature modules.
- [model.test.ts](../src/model.test.ts): Regression tests covering player
  calculations. Uses synthetic fixtures; these checks do not verify real account
  sign-in, audio output or mobile synchronization.
- [model.ts](../src/model.ts): Shared library and playback data contracts plus
  pure timing helpers. Positions and durations are seconds; browser demo books
  are fictional fixtures, not account data.
- [noteDrafts.test.ts](../src/noteDrafts.test.ts): Regression tests covering
  noteDrafts. Uses synthetic fixtures; these checks do not verify real account
  sign-in, audio output or mobile synchronization.
- [noteDrafts.ts](../src/noteDrafts.ts): Pure helpers for the earlier note-draft
  model. Preserve a draft's original timestamp while typing and avoid clearing
  newer text when a slow save finishes.
- [passages.test.ts](../src/passages.test.ts): Regression tests covering saved
  passages. Uses synthetic fixtures; these checks do not verify real account
  sign-in, audio output or mobile synchronization.
- [passages.ts](../src/passages.ts): Saved-passage types and pure
  timestamp/range validation shared by the editor. Clips contain start/end
  metadata, not recorded audio; sync state is separate from editable passage
  content.
- [playback.test.ts](../src/playback.test.ts): Regression tests covering
  official playback routing. Uses synthetic fixtures; these checks do not verify
  real account sign-in, audio output or mobile synchronization.
- [playback.ts](../src/playback.ts): Playback adapter boundary used by the UI.
  OfficialAdapter controls the native-hosted official player, DemoAdapter
  simulates time locally, and ApiPlaybackAdapter belongs to the development-only
  streaming experiment.
- [playerInfo.test.ts](../src/playerInfo.test.ts): Regression tests covering
  mini player information. Uses synthetic fixtures; these checks do not verify
  real account sign-in, audio output or mobile synchronization.
- [playerInfo.ts](../src/playerInfo.ts): Pure chapter-navigation and finish-time
  calculations. PreviousChapterIntent records rapid repeated presses separately
  from delayed telemetry so restart-then-previous behavior stays predictable.
- [previousChapter.test.ts](../src/previousChapter.test.ts): Regression tests
  covering restart and previous chapter intent. Uses synthetic fixtures; these
  checks do not verify real account sign-in, audio output or mobile
  synchronization.
- [progress.test.ts](../src/progress.test.ts): Regression tests covering saved
  position conflicts. Uses synthetic fixtures; these checks do not verify real
  account sign-in, audio output or mobile synchronization.
- [progress.ts](../src/progress.ts): Pure reconciliation for card progress
  labels. A fresh live observation can be shown directly, while mismatched
  local/cloud saved values remain a conflict rather than an invented latest
  position.
- [shortcuts.test.ts](../src/shortcuts.test.ts): Regression tests covering
  focused player shortcuts. Uses synthetic fixtures; these checks do not verify
  real account sign-in, audio output or mobile synchronization.
- [shortcuts.tsx](../src/shortcuts.tsx): Keyboard shortcut mapping and
  capability help. Ignores typing/focused controls, repeats and modifier
  combinations; global/media-key registration is handled by the native host.
- [streamEngine.test.ts](../src/streamEngine.test.ts): Regression tests covering
  protected streaming license transport. Uses synthetic fixtures; these checks
  do not verify real account sign-in, audio output or mobile synchronization.
- [streamEngine.ts](../src/streamEngine.ts): Development-only Shaka
  protected-streaming experiment. Delegates encrypted media and platform-license
  requests through a transport; no audio keys are extracted and runtime support
  is not a playable-license guarantee.
- [streamLog.test.ts](../src/streamLog.test.ts): Regression tests covering
  private playback diagnostics. Uses synthetic fixtures; these checks do not
  verify real account sign-in, audio output or mobile synchronization.
- [streamLog.ts](../src/streamLog.ts): Bounded, sanitized status messages for
  development playback diagnostics. Accepts known safe messages instead of
  preserving arbitrary service responses or signed content URLs.
- [streamTransport.ts](../src/streamTransport.ts): Transport contract for the
  development streaming experiment. Separates native IPC from the
  ordinary-browser comparison route and exposes cleanup for command
  subscriptions.
- [style.css](../src/style.css): Shared application stylesheet. Sections follow
  the existing cascade order: foundations, library/shared controls, appearance,
  compact player, workspace and responsive overrides. Reordering rules can
  change behavior even when selectors look duplicated.
- [themes.test.tsx](../src/themes.test.tsx): Regression tests covering theme
  palettes; Explore selection. Uses synthetic fixtures; these checks do not
  verify real account sign-in, audio output or mobile synchronization.
- [themes.ts](../src/themes.ts): Theme token catalog, resolution and immediate
  application. The local cache is a non-secret appearance hint; native encrypted
  settings are authoritative when available.
- [useCardPositions.ts](../src/useCardPositions.ts): Loads encrypted cached
  positions and explicitly refreshes Audible positions for library cards. Epoch
  checks and event cleanup prevent old account results from repopulating cleared
  state.
- [vite-env.d.ts](../src/vite-env.d.ts): Vite ambient type declarations for
  frontend imports and build-time environment typing; this file does not
  configure runtime account or playback behavior.
- [volume.test.ts](../src/volume.test.ts): Regression tests covering app-only
  volume. Uses synthetic fixtures; these checks do not verify real account
  sign-in, audio output or mobile synchronization.

## Root configuration

- [vite.config.ts](../vite.config.ts): Frontend build/development settings. The
  strict local port matches Tauri's development URL; production account and
  playback policy lives in native code.
