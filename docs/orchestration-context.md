# Codex Orchestrator Control Context

Date: 2026-07-01

## Purpose

This repository is building a local-first orchestration layer on top of Codex. The main conversation acts as the control room: it selects the next implementation slice, delegates work to focused Codex conversations, reviews their completion, and handles follow-up corrections, merges, cleanup, and drift control.

## Operating Model

The orchestrator thread should:

1. Keep project intent and architecture stable.
2. Choose the next smallest useful implementation slice.
3. Start worker conversations in a branch/worktree when code work should be isolated.
4. Re-ingest `docs/orchestration-learnings.md` before launching workers.
5. Give each worker a bounded task, expected deliverables, branch/commit expectations, and completion-report instructions.
6. Ask each worker to log results in the repository.
7. Ask implementation workers to commit their completed slice on a dedicated task branch unless the task is explicitly exploratory.
8. Ask each worker to send a completion prompt back to this orchestrator thread when finished.
9. Review completed work before starting dependent tasks.
10. Create correction tasks when needed.
11. Merge or clean up branches/worktrees only after review.
12. Pause and ask the user when the implementation drifts too far from the stated intention or needs a product decision.

## Reasoning-Level Guidance

Use the lowest reasoning level that fits the risk:

- Low: small mechanical edits, formatting, simple docs, simple UI polish.
- Medium: normal implementation slices, schema work, adapters with tests.
- High: architecture decisions, Codex integration changes, workflow engine design, recovery logic, security-sensitive work.
- XHigh: only for difficult cross-cutting failures or major redesigns.

## Drift Checks

After each worker completion, check:

- Does the work still center `Task` as the unit of attention?
- Are Git branches/worktrees treated as technical anchors rather than the primary product object?
- Are Codex credentials still owned by Codex rather than this app?
- Are Codex integrations still behind an adapter boundary?
- Is workflow customizability being built into the backend, not only the UI?
- Is the implementation still local-first?
- Are unreviewed branches/worktrees accumulating?

If two or more answers are "no", pause the orchestration and course-correct before continuing.

## Current State

- Repository: `andreas-kielsgaard/CodexOrchestrator`
- Local path: `C:\Users\user\Documents\Code Projects\Codex Orchestrator`
- Starting state: greenfield repository.
- Existing artifact: `docs/implementation-roadmap.md`
- This file records the orchestration protocol.
- Learning file: `docs/orchestration-learnings.md`

## Near-Term Implementation Queue

1. Bootstrap the app skeleton and developer tooling.
2. Add persistence and domain schema.
3. Add Git repo/worktree scanner.
4. Add task dashboard with seed data.
5. Add Codex JSONL run adapter.
6. Add rich Codex SDK/app-server adapter.
7. Add workflow engine and validation/review surfaces.

## Worker Completion Contract

Each worker task should finish by:

1. Running appropriate verification.
2. Writing a short result log under `docs/task-logs/`.
3. For implementation tasks, creating or using a dedicated task branch before edits.
4. For implementation tasks, committing the completed slice after verification unless the task is explicitly exploratory or blocked.
5. Leaving clear notes about changed files, verification, and unresolved issues.
6. Prompting the orchestrator thread with:

```text
Task complete: <task title>
Branch/worktree: <branch or worktree details>
Commit: <commit SHA or intentionally uncommitted reason>
Base: <main SHA or note if behind>
Git status: <git status --short --branch>
Result log: <path>
Summary: <brief summary>
Changed files: <short grouped list>
Verification: <command -> pass/fail>
Blockers: <none or exact blocker>
Needs review: <specific files, decisions, or risks>
```

## First Worker Slice

The first worker should bootstrap the project skeleton:

- Choose and initialize the app stack.
- Add basic package scripts.
- Add formatting/linting/test scaffolding.
- Add a README with setup instructions.
- Add a minimal placeholder UI or app shell.
- Avoid implementing the full domain model until the skeleton is reviewable.

This is intentionally a medium-reasoning task: it requires architectural judgment but should not spend effort on deep Codex protocol work yet.
