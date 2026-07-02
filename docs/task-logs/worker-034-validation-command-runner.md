# Worker 034 - Validation Command Runner

Date: 2026-07-02

## Worktree And Branch

- Worktree: `C:\Users\user\.codex\worktrees\d19f\Codex Orchestrator`
- Branch: `worker/034-validation-command-runner`

## Summary

- Added `src/application/validationCommandRunner.ts`, an application service boundary for running
  one configured validation command over injected stores and an injected command runtime.
- The service preflights task existence, resolves `cwd` from explicit input or linked worktree
  records, creates a running `ValidationRun`, emits `validation_started`, stores a
  `validation_log` artifact with stdout/stderr/process metadata, updates the validation run to
  `passed` or `failed`, emits `artifact_created`, and emits `validation_completed`.
- Added focused unit coverage for passing commands, non-zero failures, runtime throws, missing
  tasks, missing linked worktrees, explicit `cwd` fallback, and validation-log artifact linkage.
- Updated `docs/architecture.md` with a concise note for the new boundary.

## Decisions

- Kept process execution out of the application service. The new `ValidationCommandRuntime`
  accepts `command`, `args`, `cwd`, `env`, and chunk callbacks, and returns stdout/stderr/exit
  metadata.
- Treated exit code `0` with no signal as `passed`; all other process outcomes are `failed`.
- Runtime throws are captured as failed validation attempts with a `validation_log` artifact that
  records the launch/runtime error.
- Explicit `cwd` takes precedence over task worktree linkage. If a caller supplies an explicit
  `worktreeId`, that worktree is still validated for review metadata.

## Verification

- `git diff --check main...worker/034-validation-command-runner` - passed
- `npx vitest run src/application/validationCommandRunner.test.ts` - passed, 5 tests
- `npm run lint` - passed
- `npm run format:check` - passed
- `npm run test` - passed, 39 files / 234 tests
- `npm run build` - passed
- `npm run build:tauri` - not run; per task instructions, the known Rust/Cargo availability blocker
  remains outside this slice.

## Blockers

- None for this slice.

## Review Notes

- Review the event payload names and artifact JSON shape before UI review work consumes them.
- The service intentionally runs one command per call. Multi-command orchestration can be layered by
  a caller later without adding a workflow engine here.

## Final Git Status At Log Write

```text
## worker/034-validation-command-runner
 M docs/architecture.md
?? docs/task-logs/worker-034-validation-command-runner.md
?? src/application/validationCommandRunner.test.ts
?? src/application/validationCommandRunner.ts
```
