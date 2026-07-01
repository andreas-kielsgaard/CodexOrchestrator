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
- specific completion/reporting requirements

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

### Completion reports

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
