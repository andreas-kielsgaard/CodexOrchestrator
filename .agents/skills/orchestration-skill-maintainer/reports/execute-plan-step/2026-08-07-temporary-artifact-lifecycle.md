# Temporary artifact lifecycle

## Observation

`C:\Users\user\AppData\Local\Temp` grew by roughly 50 GB overnight to about 72.1 GB. Representative inspection found thousands of non-empty `.tmp*` trees containing copied Codex Orchestrator repositories, active SQLite fixtures, and isolated Cargo targets. Named recent targets tied the growth to repeated NCHP validation and correction work. Exact per-invocation attribution was unavailable because the temporary paths were numerous and anonymous.

## Theory

The prior storage revision addressed retained worktrees and superseded outputs visible inside task routes. It did not make the Plan Step own substantial temporary state created outside its worktree or prepare teardown for failure and interruption. Anonymous OS-temporary paths therefore survived without a durable attribution or later cleanup entry point.

## Revision

`execute-plan-step` now tells its reader to:

- use deterministic task-owned locations for substantial temporary fixtures when possible;
- establish cleanup before starting operations, including abnormal exits;
- reconcile owned leftovers when an activation resumes;
- remove superseded reproducible copies before returning; and
- report remaining task-owned state with exact external paths.

## Evaluation

The Plan Step reader creates these fixtures and is the earliest role able to name and clean them. The wording covers ordinary tests, live validation, and analogous operations without prescribing commands or assuming every dependency permits location control. Process termination can still prevent immediate cleanup, but deterministic ownership and resumed reconciliation give later activations and Slice retirement an actionable path.
