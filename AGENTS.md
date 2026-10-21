# Readability and maintenance

- Keep authored source readable. Never minify or collapse declarations, JSX,
  styles, tests or scripts. Generated build output is the exception.
- Run `npm run format` after edits and `npm run format:check` before finishing.
  These apply Prettier to supported authored files and rustfmt to Rust. Respect
  `.editorconfig`, `.prettierrc.json` and `.prettierignore`.
- Use descriptive names, one statement per line, and whitespace between logical
  steps. Prefer named function declarations for components and substantial
  helpers; small inline event handlers and callbacks are fine.
- Put app startup and window selection in `src/main.tsx`, main-screen orchestration
  in `src/App.tsx`, and feature-specific UI, hooks, models and tests together in
  `src/features/<feature>/`. Keep shared components reusable. Avoid unrelated
  folder moves during a behavior change.
- Separate persistence/network work from presentation when a component grows.
  Extract a cohesive responsibility, rather than splitting by arbitrary line count
  or adding wrappers with no purpose. Keep tests beside the code they exercise.
- Organize modules as imports, types/constants, focused helpers, then public
  components or commands. Group CSS by responsibility with clear section comments;
  preserve cascade order when reorganizing existing rules.
- Explain non-obvious intent, account boundaries, concurrency and platform quirks
  in comments. Do not narrate obvious code or put credentials/private data in examples.
- Maintain module summaries and public-contract comments when behavior changes.
  Document units, captured account/player lifetimes, side effects and failure modes.
  Keep `docs/SOURCE_INDEX.md` and `docs/DEVELOPER_GUIDE.md` aligned with source moves
  and architecture changes; do not hand-comment generated files or lockfiles.
- Preserve existing account, playback and encryption behavior during readability
  cleanups. Keep formatting changes separate from functional fixes when practical.
  Run the relevant frontend/native tests and production/release checks after moves
  or broad formatting changes.

See `CONTRIBUTING.md` for the source map and development commands.
