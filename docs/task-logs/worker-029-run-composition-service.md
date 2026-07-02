# Worker 029 Run Composition Service

Date: 2026-07-02

## Summary

Added a small application-layer service that composes the existing task-run lifecycle recorder,
injected Codex runtime, conversation store, artifact store, and event store into one persisted
non-interactive Codex run flow.

## Behavior

- Added `composeCodexTaskRun` under `src/application/`.
- Starts a task run through the lifecycle recorder before invoking Codex.
- Creates Codex conversation metadata by default and links it to the task run.
- Invokes an injected runtime using prompt, optional cwd, additional args, env, and stream callbacks
  without importing concrete runtime/process infrastructure.
- Stores structured runtime stdout JSONL as a `raw_event_stream` artifact before completing or
  failing the lifecycle.
- Emits an `artifact_created` event for the raw JSONL artifact with compact status metadata.
- Updates conversation metadata with `summary.threadId` and a concise run summary when structured
  runtime output is available.
- Completes successful runs with numeric exit code and a `final_response` artifact when Codex
  returned final agent text.
- Fails structured failed/error runs with numeric exit code and a compact status/stderr reason.
- Fails already-started runs when the runtime throws, without fabricating raw artifacts.

## Changed Files

- `src/application/runComposition.ts`
- `src/application/runComposition.test.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-029-run-composition-service.md`

## Verification

- `npm run test -- src/application/runComposition.test.ts` -> pass, 3 tests.
- `npm run lint` -> pass.
- `npm run format:check` -> pass.
- `npm run test` -> pass, 34 files / 215 tests.
- `npm run build` -> pass.
- `npm run build:tauri` -> blocked as expected because `cargo` is not installed or available on
  `PATH`.
- `git diff --check main...worker/029-run-composition-service` -> recorded in the final worker
  report after commit.

## Blockers

None known. `npm run build:tauri` remains expected to be blocked unless Rust/Cargo are installed or
available on `PATH`.

## Review Notes

- The service intentionally coordinates multiple stores without a transaction abstraction in this
  slice. Raw JSONL is stored before terminal lifecycle updates, so a later failure can still leave a
  durable stream artifact for inspection.
- The application service defines the minimal runtime shape structurally instead of importing the
  concrete Codex adapter; the existing infrastructure adapter is compatible through TypeScript
  structural typing.
