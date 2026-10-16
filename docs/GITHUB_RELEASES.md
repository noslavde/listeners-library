# GitHub builds and deployment

The `Windows installer` workflow runs on pushes to main, pull requests and manual dispatch. It runs frontend/native tests, builds the NSIS installer, applies release checks and uploads the installer, checksum, privacy notes, tester guide and dependency notices as an Actions artifact. Local account files, caches and work directories are excluded from Git and artifact packaging.

To download a test build, open Actions, select a successful Windows installer run and download `Audible-Desktop-Windows-x64` under Artifacts. Repository access is required for a private repository's downloads; private repository creation alone does not grant a friend access.

For a named release, update versions consistently in package.json/package-lock.json, Cargo.toml/Cargo.lock, tauri.conf.json and displayed version text. Commit and push, then create and push the matching `vX.Y.Z` tag. A tag mismatch fails the workflow. The tag build creates a **draft prerelease**, with the same reviewed files attached. Review its installer and complete live tests before publishing the draft. Pushing main does not publish a release.

The build uses the lockfiles, pinned official GitHub Actions and a checksum-verified native Strawberry Perl distribution to compile bundled OpenSSL/SQLCipher. The first native build can take considerable time; subsequent builds reuse native dependency caches. No Audible credentials or signing secrets are needed for this unsigned workflow. Permissions are read-only except the draft-release job's repository write permission.

Code signing and an automatic updater are not configured. No application license has been selected. Keep distribution private until release/privacy checks are complete; multiple Audible accounts do not yet have separate encrypted database partitions.

References: [Tauri GitHub build guidance](https://v2.tauri.app/distribute/pipelines/github/), [GitHub artifacts](https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts), [official Strawberry Perl download manifest](https://raw.githubusercontent.com/StrawberryPerl/strawberryperl.com/gh-pages/releases.json).
