# Tooling and build cleanup validation

Date: 2026-09-14. Branch: `cleanup/tooling-build-residue`, based on `e2bfc6c`.
Worktree: `C:/Users/user/.codex/worktrees/tooling-build-cleanup`.

## Implemented

- npm is authoritative; pnpm files removed; React build plugin classified as a development
  dependency. All 428 lockfile package entries retain their resolved versions and integrity values.
- Removed the standalone Session scenario page, simulator, and simulator/scenario tests.
- Preserved three focused screen checks and application request/navigation assertions using
  explicit responses. Kept the other review previews through fixed Session reads; simulated
  Session creation, sending, and cancellation reject explicitly.
- Renamed validation commands, removed duplicated release prerequisites, exposed separate
  App Inspector test commands, and updated current instructions.
- Corrected missing lint environments for existing documentation review scripts. Fixed a test
  clicking Plan before Session loading finished; strengthened draft reopening to assert loaded
  history and the requested Session ID.

## Completed checks

- Clean `npm ci --include=dev --no-audit --no-fund` in this new worktree.
- TypeScript check and import/re-export scan: no references into the retired directory across
  489 source/test files.
- `npm test`: 170 files, 961 tests passed. The focused Plan Builder/App orchestration rerun passed
  37 tests without React act warnings; unrelated suites still report existing act warnings.
- `npm run test:app-inspector`: 39 tests passed, none skipped, no browser test selected.
- `npm run lint`: no errors; one existing Fast Refresh warning in
  `docs/regression-review/repairs/browser.tsx`.
- Changed-file Prettier checks and `git diff --check HEAD` passed.
- `npm run validate:worktree-review`: frontend build, 46 frontend tests, 68 Rust tests, and
  Rust compilation passed. Existing Rust unused-code warnings remain.
- Default and `VITE_RUNTIME_ROOT` frontend outputs each contain only `index.html`; the isolated
  output used `.dev/isolated-runtime/dist`. The existing large-bundle warning remains.
- Headless Edge checks on the actual Vite development app: Plan Builder send shows unsupported
  feedback without creating transcript content; Harness Inspector history, model selection,
  and management disclosure work; Work Unit activity opens its recorded turn; file review shows
  the recorded diff. Screenshots are retained locally in `.dev/cleanup-validation/`.
- `npm run validate:release-build` passed; the log contains one frontend build prerequisite.
  The optimized executable was built at `src-tauri/target/release/codex-orchestrator.exe` with SHA-256
  `eddcadd002de1a8f48b6dffa949fed9475eabdf76536006f8d9c390eb0c0a6c9`.
- Native launch passed with separate `CODEX_ORCHESTRATOR_APP_DATA_DIR`,
  `CODEX_ORCHESTRATOR_WORKTREE_REVIEW_DATA_DIR`, and `WEBVIEW2_USER_DATA_FOLDER` under
  `.dev/cleanup-validation/`. The first launch omitted the WebView2 override and did not expose a
  rendered page; relaunching with its own browser-data folder resolved that limitation.
  The owner-verified WebView snapshot shows the normal empty Orchestration overview; database
  inspection found no invocations. The test instance closed through its window-close request.
  The temporary Vite server was also stopped.

The preview query routes initially show the overview because the existing application navigation
initializes before the lazy preview composition arrives. The checks opened Harness Management and
Files & diffs through their sidebar controls. No route-navigation change was made in this cleanup.

## Validation boundaries

Installer, optional installed Codex/browser contract commands, and live-provider execution are
outside this validation run. Main's existing dirty documents were preserved; no merge or push was
performed.
