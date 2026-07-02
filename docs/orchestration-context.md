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
5. Decide whether the next task should continue an existing worker conversation or start fresh based on context usefulness.
6. Summarize each new work-slice decision in the main thread before or immediately after launch.
7. Give each worker a bounded task, expected deliverables, branch/commit expectations, and explicit report-back instructions.
8. Ask each worker to log results in the repository.
9. Ask implementation workers to commit their completed slice on a dedicated task branch unless the task is explicitly exploratory.
10. Ask each worker to send a completion prompt back to this orchestrator thread when finished, and make that a separate explicit requirement in the worker launch prompt rather than relying on the report template alone.
11. Review completed work before starting dependent tasks.
12. Create correction tasks when needed.
13. Merge or clean up branches/worktrees only after review.
14. Keep worker/admin conversations visible for traceability unless the user explicitly asks to archive them.
15. Continue orchestration after each reviewed worker when there is a clear next smallest useful slice, no product decision is needed, no blocker is present, and no handoff/context-pressure trigger has fired.
16. Pause only for a concrete reason: user/product decision, blocker, review failure needing correction, context handoff, or an explicitly stated checkpoint. If pausing, state the exact reason and the next intended slice.
17. For UI-facing slices, make an explicit design-discipline decision before launch: whether the slice needs reusable component structure, component-level verification, Storybook-style isolated review, or a separate usability-review pass.
18. Use fresh usability-review worker conversations when appropriate, asking them to behave like realistic users and report confusing flows, visual bugs, missing affordances, and unexpected behavior before the UI hardens.
19. At 75% context-window usage, write a handoff report and initiate a fresh orchestration thread with `xhigh` reasoning before ending the current turn.
20. Pause and ask the user when the implementation drifts too far from the stated intention or needs a product decision.

## Reasoning-Level Guidance

Use the lowest reasoning level that fits the risk:

- Low: small mechanical edits, formatting, simple docs, simple UI polish.
- Medium: normal implementation slices, schema work, adapters with tests.
- High: architecture decisions, Codex integration changes, workflow engine design, recovery logic, security-sensitive work.
- XHigh: only for difficult cross-cutting failures or major redesigns.

## UI Design Discipline

The app should work well for users, not merely expose working code. For UI-heavy work, the orchestrator should preserve interface quality as an implementation concern:

- Prefer reusable, individually testable UI components when the UI surface grows enough to benefit from them.
- Consider tools and patterns such as Radix primitives, Storybook-style component isolation, and Vite-powered component tests when they fit the current slice; these are preferences to evaluate, not mandatory stack directives.
- Keep screens operational and scannable, especially for control-plane workflows where repeated use matters more than decorative presentation.
- Before merging substantial UI changes, consider a fresh usability-review chat that acts as a relatively clueless user or a specific user profile, navigates through the app controls, and reports confusing experiences, discrepancies, visual issues, and missing feedback.
- Treat usability-review output as review evidence: convert real issues into correction tasks, but do not let a review worker expand product scope without an orchestration decision.

## Drift Checks

After each worker completion, check:

- Does the work still center `Task` as the unit of attention?
- Are Git branches/worktrees treated as technical anchors rather than the primary product object?
- Are Codex credentials still owned by Codex rather than this app?
- Are Codex integrations still behind an adapter boundary?
- Is workflow customizability being built into the backend, not only the UI?
- Is the implementation still local-first?
- For UI changes, does the experience remain clear, reusable, testable, and usable for someone who did not build it?
- Are unreviewed branches/worktrees accumulating?

If two or more answers are "no", pause the orchestration and course-correct before continuing.

## Context Handoff

When this orchestration thread reaches 75% of its context window, the orchestrator should initiate
an active handoff, not merely write a handoff document. A handoff is not complete until a successor
thread has been created or the current thread has explicitly reported why thread initiation failed.

The orchestrator should:

1. Write a handoff report under `docs/handoffs/`.
2. Include the overall project goal, current implementation state, open worker tasks, recent merges/corrections, cleanup leftovers, known blockers, and next recommended slice.
3. Reference `docs/implementation-roadmap.md`, `docs/orchestration-context.md`, `docs/orchestration-learnings.md`, and `docs/orchestration-log.md`.
4. Commit the handoff and any orchestration-log update needed to make the transition auditable.
5. Start or hand off to a new orchestration conversation with `xhigh` reasoning using the available Codex thread tool.
6. Instruct the new orchestration thread to re-ingest the handoff report and the context/learning files before taking action.
7. Record the successor thread id in the current thread's final response, and in the repository log or handoff file when feasible without creating a circular handoff loop.
8. If the thread tool fails or is unavailable, say so explicitly in the final response and state that the handoff file is written but not initiated.
9. For any active worker, include the worker thread id or pending worktree id, expected branch, expected result log, whether the launch prompt included explicit report-back instructions, and the instruction that the successor must inspect the worker state directly if no completion prompt has arrived.
10. Keep the old orchestration and worker chats visible for traceability.

If exact context-window usage is not available, use a conservative judgment trigger: after substantial
worker cycles, when the conversation becomes difficult to audit, or before expected compaction risk.
If context compression/compaction is triggered, or the orchestrator resumes from a compressed
summary, treat that as an immediate handoff trigger and still initiate the successor thread after
stabilizing any already-started merge/logging work that would otherwise leave the repository in an
ambiguous state.

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
6. Sending the completion report back to the orchestrator thread. This is required even when the result log and commit are complete.

Every worker launch prompt must include a dedicated "Report back" instruction, not just the report
shape. Use wording like:

```text
When complete, send a new message back to the orchestration/control-room thread with the completion
report below. If your environment cannot cross-post to that thread, say that explicitly in your
final worker-thread message and still include the full report so the orchestrator can retrieve it.
```

The completion report should use this shape:

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
