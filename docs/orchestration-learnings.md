# Orchestration Learnings

Date started: 2026-07-01

This file records practical lessons from running the implementation as a control-room conversation plus delegated worker conversations. Re-ingest this file alongside `docs/orchestration-context.md` and `docs/implementation-roadmap.md` before launching new workers and during periodic drift reviews.

## Standing Rules

### Worker branches and commits

Code-changing workers should not leave integration as an avoidable admin chore.

For implementation tasks, the worker prompt should require the worker to:

1. Create or use a dedicated task branch in its worktree before editing.
2. Commit its completed slice when verification is done.
3. Include the branch name, commit SHA, and `git status --short --branch` result in its completion report.
4. Leave changes uncommitted only when the task was explicitly exploratory or when committing would hide an unresolved issue.

If a Codex-created worktree starts in detached HEAD, the worker should create a branch before edits, for example:

```text
git switch -c worker/001-bootstrap
```

### Context discipline

Worker prompts should pass compact visible context only:

- the current task brief
- relevant user instructions
- pointers to project docs
- specific completion/reporting requirements, including an explicit instruction to report back to
  the orchestration/control-room thread when finished

Do not serialize hidden model context into files or prompts.

Before launching each task, decide whether prior worker context is useful:

- Start a fresh conversation when the next task is independent, when old implementation details could bias the worker, or when the previous worker's context is mostly noise.
- Continue an existing worker conversation when the next task directly corrects, extends, or debugs that worker's own changes and its local context is valuable.
- Record the choice in the orchestration log so context carryover is intentional.

When launching a new work slice, summarize the orchestration decision in the main thread. The summary should say:

- what the work slice is attempting to accomplish
- why it is the next useful slice
- whether the worker is fresh or continued, and why
- what reasoning level is being used and why
- what branch/worktree shape is expected
- what completion/review signal will count as success

After a worker is reviewed, merged, verified, logged, and Git-cleaned, the orchestrator should continue directly to the next smallest useful slice when all of these are true:

- the next slice is clear from the roadmap, handoff, or just-completed work
- no product/user decision is needed
- there is no blocker or failed review requiring correction
- context pressure has not reached the handoff threshold

Do not stop merely because the repo is at a clean checkpoint. A clean checkpoint is the place to make the next orchestration decision. Pause only for a concrete reason, and if pausing, state that reason plus the next intended slice.

At 75% context-window usage, the orchestrator should write a handoff report and initiate a new orchestration thread with `xhigh` reasoning. The handoff report should live under `docs/handoffs/` and point to:

- `docs/implementation-roadmap.md`
- `docs/orchestration-context.md`
- `docs/orchestration-learnings.md`
- `docs/orchestration-log.md`
- current worker result logs under `docs/task-logs/`

The new orchestration thread prompt should include the handoff report path, explicitly say it is taking over orchestration, and require re-ingesting the overall task overview and learnings before launching or reviewing work. The handoff is not complete merely because the report exists; after committing the report, use the available Codex thread tool to create or hand off to the successor thread, record the successor thread id when feasible, and tell the user explicitly if tool failure prevented initiation. If exact context usage is unavailable, trigger conservatively when the thread becomes large enough that auditability or compaction risk is a concern. If context compression/compaction is triggered, or the orchestrator resumes from a compressed summary, hand off immediately instead of continuing to accumulate orchestration state in the compressed thread.

### Completion reports

Report-back is part of the worker contract, not just the report's format. Every worker launch
prompt should contain a dedicated "Report back" section telling the worker to send a new message
back to the orchestration/control-room thread when complete. If a direct cross-post is unavailable,
the worker should say so explicitly in its final worker-thread message and still include the full
report there. For active workers listed in a handoff, include the worker thread id or pending
worktree id, expected branch, expected result log, and whether explicit report-back instructions
were included; the successor must inspect worker state directly if no completion prompt has
arrived.

Every worker completion should include:

- task title
- branch/worktree path
- commit SHA, unless intentionally uncommitted
- exact branch state, including whether it includes current `main`
- `git status --short --branch`
- result log path
- summary
- verification commands and pass/fail results
- changed-file summary
- known blockers with exact missing dependency or failing command
- specific review points tied to files, decisions, or risks

Keep reports concise. Avoid generic review prompts that do not tell the orchestrator what to inspect.

Preferred report shape:

```text
Task complete: <task title>
Branch/worktree: <branch and path>
Commit: <sha or intentionally uncommitted reason>
Base: <main sha or note if behind>
Git status: <short status>
Result log: <path>
Changed files: <short grouped list>
Verification: <command -> pass/fail>
Blockers: <none or exact blocker>
Needs review: <specific files/decisions/risks>
```

### UI design discipline and usability review

The user values strong UI design discipline and is still building their own UI-design judgment, so the orchestration process should protect the user experience early instead of treating polish as a final pass.

For UI-facing implementation slices, the orchestrator should:

- Decide explicitly whether the slice needs reusable component boundaries, component-level tests, isolated component review, or Storybook-style stories.
- Consider Radix primitives, Storybook, and Vite-based component workflows when they naturally support reusable and individually testable UI, without treating them as mandatory technology choices.
- Keep UI worker prompts focused on the target user's workflow, density, scanability, affordances, states, and feedback.
- Avoid letting implementation workers ship only the happy-path surface; ask for empty/loading/error/disabled/overflow states when the control naturally needs them.
- Before or after substantial UI slices, consider launching a fresh usability-review worker that does not implement code and instead acts like a relatively clueless user or a named user profile. That worker should navigate the app with available UI controls, report bugs, confusing copy, missing feedback, visual hierarchy problems, and expectation mismatches.
- Convert usability-review findings into bounded correction tasks. Keep the reviewer chat visible for traceability.

This is an orchestration preference, not a hard stack directive. The project can adopt component tooling incrementally when the UI surface justifies it.

### Admin work

If integration, merge, cleanup, or branch repair becomes nontrivial, consider using a dedicated admin worker conversation rather than embedding the work inside an implementation task.

Do not archive worker or admin conversations by default. The user wants those chats visible as part of the trace of the orchestration work. Archive only when the user explicitly asks for it.

For Windows worktree cleanup:

1. Verify the worktree is clean and merged.
2. Archive the completed worker thread first if appropriate.
3. Run `git worktree remove` from the main checkout.
4. Delete the merged branch only after the worktree is unregistered or known to be safe.
5. If an empty directory remains locked, log it and retry later instead of escalating blindly.

## Incident Notes

### 2026-07-01: Worker 001 left bootstrap changes uncommitted

Worker 001 successfully bootstrapped the app skeleton but left changes uncommitted in a detached Codex worktree. This was predictable because the worker prompt did not explicitly require branch creation or a final commit.

Correction:

- Update the worker completion contract to require task branch creation and final commits for implementation work.
- Future workers should report their commit SHA.
- Future workers should report whether their branch includes the current `main` and provide `git status --short --branch`.
- Future prompts should ask for specific changed-file and verification summaries, not broad "please review everything" notes.
- The orchestrator should still review before merging, but should not have to reconstruct the worker's intended integration unit.

### 2026-07-01: Windows held an empty Codex worktree directory locked after cleanup

After Worker 001 was merged, `git worktree remove --force` unregistered the worktree but Windows denied deletion of the empty worktree directory. The merged branch was deleted successfully.

Correction:

- Treat worktree removal and branch deletion as separate admin steps.
- Check `git worktree list --porcelain`, `git branch --merged main`, and the physical directory state after cleanup.
- If only an empty locked directory remains, record the path and retry later rather than force-killing unknown processes.

### 2026-07-01: Worker chats were archived too aggressively

The orchestrator archived Worker 001 and Worker 002 after merge/cleanup. The user clarified that worker chats should remain visible so the work can be traced.

Correction:

- Worker 001 and Worker 002 were unarchived.
- Future worker/admin conversations should remain visible unless the user explicitly asks to archive them.
- Cleanup should focus on Git branches/worktrees and logs, not hiding the conversation trail.

### 2026-07-02: Worker 005 initially wrote implementation files in the main checkout

After Worker 005 launched in a Codex worktree, untracked implementation files appeared in the main checkout while the assigned worker worktree was still detached and clean. The orchestrator sent an immediate correction to the worker thread requiring Worker 005 to work only in its assigned worktree and leave the main checkout clean.

Correction:

- Worker prompts should clearly distinguish the saved project path from the assigned Codex worktree path when both are visible.
- When a worker thread is created in a Codex worktree, the worker should treat the thread `cwd` or explicitly assigned worktree path as authoritative for edits.
- The orchestrator should check both the main checkout and worker worktree shortly after launch when a worker's first status messages suggest it may have changed directories.

### 2026-07-02: Orchestrator paused after Worker 006 despite a clear next slice

After Worker 005 and Worker 006 were reviewed, merged, verified, logged, and cleaned up, the orchestrator identified the next useful slice but stopped instead of launching it. The user correctly pointed out that the operating model expects continued orchestration unless there is a concrete pause reason.

Correction:

- A clean post-merge checkpoint is not by itself a reason to pause.
- If the next smallest useful slice is clear and no decision/blocker/handoff trigger exists, summarize the slice and continue.
- If pausing anyway, state the exact pause reason and the next intended slice so the user can challenge the decision.

### 2026-07-02: Worker 015 finished without reporting back to orchestration

Worker 015 was launched with a completion report shape but without an explicit instruction to send
that report back to the orchestration/control-room thread. The worker completed and committed its
branch, but the orchestrator had to discover completion by inspecting the worktree.

Correction:

- Completion report shape is not enough; every worker launch prompt needs a dedicated report-back
  instruction.
- The prompt should ask the worker to send a new message to the orchestration/control-room thread
  when finished, and to state clearly in its own final message if cross-posting is unavailable.
- Handoff reports for active workers should say whether explicit report-back instructions were
  included, so the successor knows whether to wait for a prompt or inspect the worker directly.
- The orchestrator should still periodically inspect active worker worktrees instead of relying
  solely on message delivery.

### 2026-07-02: Handoff report was written but successor thread was not initiated

After Worker 018 was reviewed, merged, verified, logged, and Git-cleaned, the orchestration thread
correctly wrote a compaction handoff report but stopped before creating or handing off to the
successor orchestration thread. The user clarified that a handoff must be active, not just
documentary.

Correction:

- Handoff reports are necessary but insufficient; the orchestrator must also initiate the successor
  thread with the handoff prompt before ending the turn.
- The successor prompt should use `xhigh` reasoning when the handoff is caused by context pressure
  or compaction.
- If thread tooling is unavailable or fails, the orchestrator must explicitly report that the
  handoff is written but not initiated, with the exact blocker.
- Future handoff instructions should include the successor-thread creation step, not only the file
  writing step.
