# Worker 025 Task Run Lifecycle Recorder

Date: 2026-07-02

## Summary

Added a small pure TypeScript application-layer recorder that coordinates existing store boundaries
for task-run lifecycle persistence. This slice does not start Codex, parse Codex output, run
validation commands, open databases, import SQLite, add Tauri/Rust commands, touch React/UI, add
dependencies, or introduce transaction abstractions.

## Behavior

- Added `startTaskRunLifecycle` to preflight task existence, create a running `TaskRun`, optionally
  create and link Codex conversation metadata, update the task to `running` and
  `waiting_on_agent`, preserve existing task conversation IDs, and append a `run_started` event.
- Added `completeTaskRunLifecycle` to mark a run completed, move the task to `completed` and
  `needs_review`, optionally create a `final_response` artifact, and append a `run_completed`
  event with linked IDs and an outcome payload.
- Added `failTaskRunLifecycle` to mark a run failed, move the task to `failed` and
  `needs_action_now`, and append a `run_completed` event with failure outcome details.
- Added a typed `TaskRunLifecycleTaskNotFoundError` for missing-task preflight failures.
- Kept all dependencies on existing domain store interfaces so runtime adapters can compose this
  service without depending on concrete SQLite classes.

## Changed Files

- `src/application/taskRunLifecycle.ts`: lifecycle recorder service functions, input/output types,
  dependency interface, and missing-task error.
- `src/application/taskRunLifecycle.test.ts`: in-memory store coverage for start, conversation-link
  preservation, successful completion with final artifact, failure completion, missing-task
  preflight behavior, and emitted events.
- `docs/architecture.md`: documented the lifecycle recorder application boundary.
- `docs/task-logs/worker-025-task-run-lifecycle-recorder.md`: recorded this worker result.

## Verification

- Focused lifecycle tests pass locally.
- Full required verification is recorded in the worker completion report after final run.

## Blockers

None known.

## Review Notes

- The recorder intentionally coordinates multiple stores without an atomic transaction abstraction.
  Future runtime/database wiring can wrap the underlying concrete stores when they share one durable
  connection.
- Completion inputs require both `taskId` and `taskRunId` because the current `TaskRunStore` query
  boundary does not expose a direct lookup by run ID.
