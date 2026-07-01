# Orchestration Handoff

Date: 2026-07-02

## Why This Handoff Exists

The original orchestration thread has accumulated several worker cycles and substantial process context. Per the orchestration behavior rules, this handoff preserves auditability before context compression or context-window pressure makes the control thread harder to reason about.

The next orchestration thread should use `xhigh` reasoning, re-ingest the linked project context files, and continue as the control room for the project.

## Project Goal

Build a local-first Codex Orchestrator: a customizable UI and workflow control plane on top of Codex, Git repositories, branches, worktrees, task attention states, conversations, validation logs, and review artifacts.

The app should keep Codex as the execution engine and should not read or manage Codex credentials. The orchestrator owns project/task/worktree/conversation state, workflow definitions, Git orchestration, review state, and UI customization.

## Required Context Files

The new orchestration thread should read these before taking action:

- `docs/implementation-roadmap.md`
- `docs/orchestration-context.md`
- `docs/orchestration-learnings.md`
- `docs/orchestration-log.md`
- `docs/task-logs/worker-001-bootstrap.md`
- `docs/task-logs/worker-002-domain-model.md`
- `docs/task-logs/worker-003-git-adapter-foundation.md`
- `docs/task-logs/worker-004-git-scan-mapping.md`

## Current Git State

- Repository path: `C:\Users\user\Documents\Code Projects\Codex Orchestrator`
- Current branch: `main`
- Current commit at handoff: `f8745b7debc4f293d3235875364d0f5b9fe9b85b`
- Git status at handoff: clean
- Registered Git worktrees at handoff: only the main worktree

## Completed Worker Slices

### Worker 001: Bootstrap App Skeleton

- Result: merged
- Worker thread: `019f1f86-6f0d-7753-a1d1-f6fd3414a67d`
- Worker commit: `07f2c63`
- Merge commit: `5a246f2`
- Outcome: Tauri v2 + React + TypeScript + Vite skeleton, npm scripts, ESLint/Prettier/Vitest, minimal Tauri command boundary, README, architecture note, and Open Tasks dashboard placeholder.
- Review correction: dashboard CSS density tightened so all five attention columns fit at 1280px.
- Verification: frontend lint, format, tests, and build passed.
- Known blocker: `npm run build:tauri` fails because Rust/Cargo are not installed or not on `PATH`.

### Worker 002: Domain Model And Dashboard Projection

- Result: merged
- Worker thread: `019f1f99-5c3f-7961-874e-6af58a7c3ec5`
- Worker commit: `8550e77`
- Merge commit: `39b4eb4`
- Outcome: TypeScript domain model, seed records, dashboard projection, tests, and docs.
- Review correction: changed `Task.conversationId` to `Task.conversationIds`; corrected Worker 002 seed thread id.
- Accepted decision: `running` plus `waiting_on_agent` maps to `Working`; non-running `waiting_on_agent` maps to `Waiting`.
- Verification: frontend lint, format, tests, and build passed.

### Worker 003: Git Adapter Foundation

- Result: merged
- Worker thread: `019f1fa8-69b6-7c52-86e5-c72fc8b8aaf4`
- Worker commit: `10b6f39`
- Merge commit: `9d9ed99`
- Outcome: pure TypeScript parsers/types for Git status porcelain v1 `-z`, branch summary format, worktree porcelain `-z`, and future adapter boundary.
- Review correction: all porcelain v1 unmerged status pairs (`DD`, `AU`, `UD`, `UA`, `DU`, `AA`, `UU`) classify as `unmerged`.
- Accepted decision: parser output normalizes Windows separators to forward slashes for stable downstream comparison.
- Verification: frontend lint, format, tests, and build passed.

### Worker 004: Git Scan Mapping

- Result: merged
- Worker thread: `019f1fb1-15ca-7713-bdec-a003aa24abb1`
- Worker commit: `806a0ad`
- Merge commit: `2d09ce3`
- Outcome: pure TypeScript Git scan assembly, `git remote -v` parsing, domain-facing scan facts, and tests.
- Review correction: `GitRepoDomainFacts.defaultBranch` is optional; mapper no longer invents `main` when scan facts cannot identify a default branch.
- Accepted decision: non-root worktrees use `dirtyState: "unknown"` because root scans only include root status output.
- Verification: frontend lint, format, tests, and build passed.

## Orchestration Behavior Learnings To Preserve

- Pass compact visible context only: task brief, relevant user instructions, pointers to docs, and concrete completion/reporting requirements.
- Before launching work, decide whether to continue an existing worker or start fresh based on context usefulness.
- Summarize every new work slice in the orchestration thread: goal, why next, fresh/continued context, reasoning level, branch/worktree intent, and success signal.
- Implementation workers should create/use a dedicated branch, commit their finished slice, and report branch, commit SHA, base/main status, result log, verification, blockers, and specific review points.
- Worker/admin chats should remain visible for traceability. Do not archive them unless the user explicitly asks.
- If context reaches 75%, or if context compression/compaction is triggered, write a handoff report and start a fresh `xhigh` orchestration thread.
- Cleanup should remove Git worktree registrations and merged branches when safe, but avoid force-killing unknown Windows locks.

## Cleanup State

Git only tracks the main worktree now.

Physical Codex worktree folders may still remain locked by Windows even after Git worktree registration was removed:

- `C:\Users\user\.codex\worktrees\15ac\Codex Orchestrator`
- `C:\Users\user\.codex\worktrees\eaeb\Codex Orchestrator`
- `C:\Users\user\.codex\worktrees\af5b\Codex Orchestrator`
- `C:\Users\user\.codex\worktrees\fe6a\Codex Orchestrator`

Do not aggressively force-delete them. Retry later if needed after the app releases handles.

## Current Implementation State

The repo now has:

- React/Vite/Tauri skeleton.
- Attention-first Open Tasks dashboard.
- Domain model and dashboard projection.
- Seed/demo domain records.
- Git parser foundation.
- Git scan assembly and domain-facing mapping facts.
- Tests for domain projection and Git parsing/mapping.

The project intentionally does not yet have:

- SQLite persistence or migrations.
- Tauri/Rust database commands.
- Tauri/Rust Git command execution.
- Codex CLI/SDK/app-server integration.
- Repo/project/branch drilldown UI.
- Workflow engine.
- Validation/review surface beyond foundational logs/tests.

## Known Blockers

- Rust/Cargo are missing or not on `PATH`, so `npm run build:tauri` is blocked.
- Until Rust/Cargo is installed, prefer pure TypeScript slices or explicitly log that Tauri verification is blocked.

## Recommended Next Slice

Recommended next slice: introduce a persistence-neutral project/repo registry service in TypeScript that can accept domain records plus Git scan facts and produce upsert plans for projects, repos, branches, and worktrees.

Why this is next:

- Worker 004 provides Git scan facts, but the app still lacks the sync/planning layer that decides how discovered Git facts update the app's domain records.
- This can stay pure TypeScript and testable while Rust/Cargo remains blocked.
- It prepares the later SQLite/Tauri persistence slice without needing database migrations yet.

Suggested scope:

- Create pure sync/upsert planning functions for `Repo`, `Branch`, and `Worktree`.
- Use existing `DomainRecords` and `GitRepoScanDomainFacts`.
- Do not persist to SQLite yet.
- Add tests for new repo discovery, branch updates, worktree dirty-state updates, stale/missing worktree handling, and preserving app-owned fields such as branch intent.
- Write a worker result log.

Suggested worker setup:

- Fresh worker conversation.
- Reasoning: medium.
- Branch: `worker/005-repo-sync-planning`.
- Keep chat visible.

## Handoff Instruction For New Orchestrator

You are taking over as the orchestration/control-room thread. Before launching or reviewing any worker, read the required context files listed above. Continue using the same operating model:

- choose the next slice,
- summarize orchestration considerations in the main thread,
- launch bounded worker conversations in branches/worktrees,
- review their results,
- merge or correct,
- keep chats visible,
- log outcomes,
- monitor drift,
- hand off again before context pressure or compaction makes the thread hard to audit.
