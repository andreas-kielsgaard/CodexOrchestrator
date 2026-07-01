# Worker 002 Domain Model And Dashboard Projection

Date: 2026-07-01

## Summary

Added the TypeScript domain model foundation for Projects, Repos, Branches, Worktrees, Conversations, Tasks, TaskRuns, Artifacts, ValidationRuns, and Events. Replaced the hardcoded dashboard groups with seed/demo domain records plus projection logic that derives the Open Tasks dashboard groups.

Execution state and attention state are separate task fields. The projection keeps completed tasks open when they need review and treats a running task waiting on an agent as operationally working.

## Changed Files

- `src/domain/model.ts`: core domain records and state unions.
- `src/domain/dashboardProjection.ts`: dashboard group projection and grouping rules.
- `src/domain/seedData.ts`: linked seed/demo records for the dashboard.
- `src/domain/taskDashboard.ts`: thin export of projected seed dashboard data.
- `src/domain/dashboardProjection.test.ts`: projection and state-case tests.
- `src/app/App.tsx`: renders projected attention state.
- `README.md` and `docs/architecture.md`: domain boundary notes.

## Verification

- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Scope Boundaries

- SQLite persistence was not implemented.
- Tauri Rust database commands were not implemented.
- Codex CLI, SDK, and app-server integration were not implemented.
- React components consume projection output and do not own grouping rules directly.

## Branch Notes

- Branch: `worker/002-domain-model`
- Worktree: `C:\Users\user\.codex\worktrees\eaeb\Codex Orchestrator`
- Launch base: `4d9af770677d444122cac31a3e18876ff051933b`
- Local `main` at verification time: `9f0bf0f808f33d0beb4a5b3d2f1aee3fc1f241b1`
- This branch is based on the launch base and is behind current local `main`.

## Blockers

None.

## Needs Review

- `src/domain/dashboardProjection.ts`: confirm the intended bucket for `running` plus `waiting_on_agent`. This slice treats it as `Working` because the agent is actively executing, while non-running `waiting_on_agent` tasks go to `Waiting`.
- `src/domain/model.ts`: confirm whether future persistence should keep the same field names before SQLite migrations lock them in.

## Orchestrator Review Addendum

The orchestrator accepted the `running` plus `waiting_on_agent` bucket decision: it belongs in `Working` because the agent is actively executing, while non-running `waiting_on_agent` tasks belong in `Waiting`.

The orchestrator made one schema-shape correction before merge: `Task.conversationId` was changed to `Task.conversationIds` so a task can link to multiple conversations, matching the project model. The seed `Conversation.externalThreadId` was also corrected to the Worker 002 thread id.
