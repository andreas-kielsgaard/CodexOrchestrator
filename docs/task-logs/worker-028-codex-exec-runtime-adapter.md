# Worker 028 Codex Exec Runtime Adapter

Date: 2026-07-02

## Summary

Added a narrow local runtime adapter for non-interactive `codex exec --json` execution. The adapter
invokes Codex through an injectable process runner, captures raw JSONL stdout and stderr, parses and
summarizes the stdout stream with the existing Codex JSONL parser, and returns terminal metadata for
later task-run composition.

## Behavior

- Added `createCodexRuntime` and `runCodexExec` under `src/infrastructure/codex/`.
- Builds command arguments as `['exec', '--json', ...additionalArgs, prompt]`.
- Includes a default Node runner using `node:child_process.spawn`.
- Treats configured environment values as overrides over the inherited process environment.
- Supports optional stdout/stderr chunk callbacks for minimal stream observation.
- Preserves raw stdout JSONL and stderr on every structured result.
- Returns exit code, signal, parsed events, JSONL summary, and a `completed`/`failed`/`error`
  classification.
- Returns structured failed results for non-zero Codex exits when stdout is parseable.
- Throws for runner/launch failures and JSONL parse failures, where the adapter cannot return a
  trustworthy parsed result.

## Out Of Scope

- Does not compose task-run lifecycle state.
- Does not persist artifacts, conversations, events, or validation records.
- Does not collect diffs, run validation commands, manage worktrees, or wire React/Tauri UI.
- Does not read or manage Codex credentials.

## Changed Files

- `src/infrastructure/codex/codexRuntime.ts`: runtime adapter, process runner boundary, default Node
  runner, argument builder, and result classification.
- `src/infrastructure/codex/codexRuntime.test.ts`: focused adapter tests using an injected fake
  process runner.
- `docs/architecture.md`: documented the runtime adapter boundary.
- `docs/task-logs/worker-028-codex-exec-runtime-adapter.md`: recorded this worker result.

## Verification

- `git diff --check main...worker/028-codex-exec-runtime-adapter` -> pass
- `npm run test -- src/infrastructure/codex/jsonlEvents.test.ts src/infrastructure/codex/codexRuntime.test.ts`
  -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass, 33 test files / 210 tests
- `npm run build` -> pass

- `npm run build:tauri` -> blocked as expected in this environment because `cargo` is not installed
  or available on `PATH`.

## Live CLI Smoke

Skipped live `codex exec --json` smoke because `codex --help` fails in this worker environment with
`Access is denied` when launching `codex.exe` from the WindowsApps package path.

## Blockers

None in implementation.

## Review Notes

- Classification prioritizes Codex JSONL terminal `error`/`turn.failed` events before process exit
  metadata. A non-zero process exit with parseable JSONL returns `failed` even if a
  `turn.completed` event is present.
- A zero-exit stream without a terminal JSONL event is classified as `failed` with a clear status
  reason so later composition can decide how to surface incomplete output.
- `additionalArgs` are placed before the prompt to match `codex exec [OPTIONS] [PROMPT]`.

## Orchestrator Review Addendum

- Added focused tests for two reported classification edges: parseable output with no terminal
  event, and process signal exit after parseable completed JSONL.
- Adjusted the fake process runner test helper so explicit `null` exit codes can be tested instead
  of being replaced by the default zero exit code.
