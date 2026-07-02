# Worker 036 - Validation Command Runtime Adapter

Date: 2026-07-02

## Summary

- Added a Node-side validation command runtime adapter at
  `src/infrastructure/validation/validationCommandRuntime.ts`.
- Added focused tests for runner forwarding, empty args defaults, stdout/stderr accumulation and
  chunk callbacks, non-zero exits returning normally, real Node spawn cwd/env/args behavior, and
  launch failure propagation.
- Updated `docs/architecture.md` with the validation runtime adapter boundary.

## Decisions

- Kept the adapter structurally compatible with the application-layer `ValidationCommandRuntime`
  using type-only imports so infrastructure has no runtime application dependency.
- Mirrored the Codex runtime runner seam: `createValidationCommandRuntime` accepts an injectable
  runner, while `createNodeValidationCommandProcessRunner` uses `node:child_process.spawn`.
- Kept validation status classification out of the adapter. Exit code and signal are returned as
  process metadata for `validationCommandRunner.ts` to classify.
- Used `shell: false`, argument arrays, `windowsHide: true`, and inherited `process.env` with
  caller-provided env overrides.

## Verification

- `npm run test -- src/infrastructure/validation/validationCommandRuntime.test.ts` - passed
  (7 tests).
- `npm run lint` - passed.
- `npm run format:check` - passed.
- `npm run test` - passed (41 files, 247 tests). Existing Node SQLite experimental warnings were
  emitted during SQLite tests.
- `npm run build` - passed.
- `git diff --check main...worker/036-validation-command-runtime-adapter` - passed.

## Blockers

- None for this slice.
- `npm run build:tauri` was not run per task instruction; the known Rust/Cargo availability blocker
  remains relevant for that command.

## Review Notes

- The concrete adapter is not composed into `validationCommandRunner.ts`; runtime wiring remains a
  later slice.
- The adapter executes one configured command per call and intentionally does not add multi-command
  orchestration.

## Final Git Status

Expected after commit:

```text
## worker/036-validation-command-runtime-adapter
```
