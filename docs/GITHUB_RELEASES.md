# GitHub builds and deployment

The `Windows installer` workflow runs on pushes to main, pull requests, published releases and manual dispatch. It runs frontend/native tests, builds the NSIS installer, applies release checks and uploads the installer, checksum, privacy notes, tester guide and dependency notices as an Actions artifact. Local account files, caches and work directories are excluded from Git and artifact packaging.

To download a test build, open Actions, select a successful Windows installer run and download `Listeners-Library-Windows-x64` under Artifacts. Repository access is required for a private repository's downloads; private repository creation alone does not grant a friend access.

For a named release, update versions consistently in package.json/package-lock.json, Cargo.toml/Cargo.lock, tauri.conf.json and displayed version text. Commit and push, then create and push the matching `vX.Y.Z` tag. A version-tag mismatch fails the workflow. The tag build creates a **draft prerelease**, with the reviewed files attached. Review its installer and complete live tests before publishing the draft. Pushing main does not publish a release.

If you instead publish a release using GitHub's release form, the published event builds its tagged source and attaches the files to that existing release. Custom tag names such as `release_1` are supported for this route. Tags starting with v followed by a number still must match the version in the source. The installer appears under Assets after the workflow succeeds; a published page can initially show source archives only. Those automatic Source code archives are not installers.

To repair an older release, open Actions → Windows installer → Run workflow. Choose main and enter the existing release tag in **release_tag**. The workflow builds that tag's source rather than silently attaching main. An empty field makes a test package without publishing a release. Old tags created before this workflow update may need this manual route. An explicit repair requires an existing readable release; a read/authentication error does not create another release.

Existing titles, notes, published/draft status and assets are preserved. The job uploads missing files only, so reruns do not fail on duplicate names or overwrite an existing installer. If an installer already exists, a missing checksum is generated from the actual published file. Changed binaries require a new version/release. Tag and release events for the same tag are serialized.

For manual uploads, edit the release and use **Assets / Attach binaries**, not the description's attachment toolbar. Upload the Windows Setup.exe, SHA256SUMS.txt, TESTER_GUIDE.md, PRIVACY.md and THIRD_PARTY_LICENSES.txt, then Update release. Testers download Setup.exe under Assets. Private repositories require access; the release title does not change repository visibility.

Run `node scripts/check-release-workflow.mjs` to test the actual attachment shell step with a mocked GitHub command. Checks cover draft creation, missing-release failure, missing uploads, published-file checksums and safe reruns, without real credentials or network access.

The build uses the lockfiles, pinned official GitHub Actions and a checksum-verified native Strawberry Perl distribution to compile bundled OpenSSL/SQLCipher. The first native build can take considerable time; subsequent builds reuse native dependency caches. No Audible credentials or signing secrets are needed for this unsigned workflow. Permissions are read-only except the release-assets job's repository write permission.

Code signing and an automatic updater are not configured. The application source is MIT licensed. Keep this labeled as a beta; live account-switching and sign-out cleanup checks still require an installed-build test.

References: [Tauri GitHub build guidance](https://v2.tauri.app/distribute/pipelines/github/), [GitHub artifacts](https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts), [official Strawberry Perl download manifest](https://raw.githubusercontent.com/StrawberryPerl/strawberryperl.com/gh-pages/releases.json).
