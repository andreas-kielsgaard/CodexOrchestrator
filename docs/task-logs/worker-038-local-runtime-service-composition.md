# Worker 038: Local Runtime Service Composition

Date: 2026-07-02

Branch: `worker/038-local-runtime-service-composition`

## Summary

Added a narrow Node-only local runtime composition boundary at
`src/infrastructure/localRuntimeComposition.ts`. The boundary opens the local app SQLite database,
reuses the opened app store bundle, wires concrete local Git/Codex/validation runtime adapters to
the already-merged application services, and exposes close/dispose behavior for the database.

The composition exposes:

- store-backed Open Tasks dashboard client
- task-run lifecycle recorder
- Codex run composition service
- repo registry scan service
- task worktree selection service
- diff collection service
- validation command runner service

## Verification Notes

- Added focused tests in `src/infrastructure/localRuntimeComposition.test.ts`.
- The test uses an in-memory SQLite database and injected fake Git/Codex/validation process runners,
  so no live Git, Codex, or validation commands run.
- The test proves the composed services share the same opened store bundle by creating a task,
  composing a Codex run, collecting a diff, running validation, and reading artifacts/events/runs
  back through the shared stores.

## Decisions

- Kept this as infrastructure-only TypeScript composition. No UI, Tauri commands, Rust code, or
  workflow engine behavior was added.
- Kept React/browser entrypoints away from the Node-only module.
- Left repo registry list/remove behavior, post-run triggers, branch naming policy, cleanup policy,
  and live supervision to later slices.

## Blockers

- None for this slice.
- `npm run build:tauri` remains outside scope because Rust/Cargo are known unavailable on `PATH`.
