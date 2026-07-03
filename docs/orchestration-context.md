# Codex Orchestrator Control Context

## Purpose

This repository is building a local-first control plane for Codex-driven work. The orchestration
thread acts as the control room: choose the next useful slice, delegate focused worker tasks,
review results, merge accepted work, keep state recoverable, and prevent product drift.

## Authoritative Rules

### Keep Work Moving

- Choose the next smallest useful slice from `docs/first-slice-completion-plan.md`,
  `docs/implementation-roadmap.md`, and `docs/active-task-map.md`.
- Continue after a clean reviewed merge when the next slice is clear and no blocker, user decision,
  or review correction is pending.
- Pause only for a concrete reason: product decision, blocker, failed review, unclear next slice,
  drift from project intent, or explicit user request. State the reason and the likely next step.
- Consult the first-slice-completion-plan and active-task-map when deciding on the next task(s) to start.

### Fast Report Intake

When a worker reports completion, do a quick intake before broad context refresh or historical
logging:

1. Read the worker report, result log, branch state, and touched-file summary.
2. Classify the branch as ready for review, needs small orchestrator correction, needs worker
   follow-up, blocked, or rejected.
3. If it is ready or only needs a small correction, review and verify it immediately.
4. If the next slice is independent of that review result, launch or continue it in parallel instead
   of waiting for merge cleanup.

Do not scan `docs/orchestration-log.md` or rewrite long state summaries just because a report
arrived. Use `docs/active-task-map.md` for current unresolved state and update it after the
operation settles.

### Delegate Clearly

Every implementation worker prompt must include:

- task goal
- explicit boundaries for meaningful work that is intentionally deferred
- context critical to intelligent completion of the task
- targeted branch/worktree
- targeted result log under `docs/task-logs/`
- proposed verification commands
- requirement to commit the completed slice unless blocked or exploratory
- a dedicated "Report back" instruction telling the worker to message the orchestration thread when
  complete

Workers should receive enough context to act well, not a transcript dump.

Process reflection from user discussion: tier context instead of preloading it. Prefer rediscovery
from git, source, worker threads, and result logs when details are missing. Result logs are fallback
evidence, not routine worker input; orchestration recovery/process notes stay mostly
orchestrator-owned unless they directly affect the task.

Prefer worker prompts that point to the smallest authoritative docs and name the exact decision the
worker owns. Put speculative background, old incidents, and unrelated roadmap history in the result
log or orchestration log only when they change the current decision.

### Review Before Merge

- Inspect worker output, result log, and branch state before merging.
- Run independent verification appropriate to the change.
- Make small orchestrator corrections on the worker branch when that is faster and clearer than
  relaunching.
- Merge only accepted work.
- After merge, verify on `main`, update logs/maps, then clean branch/worktree state when safe.
- Leave worker/admin chats visible unless the user explicitly asks to archive them.

### Keep Recovery Cheap

`docs/active-task-map.md` is the recovery surface. `docs/orchestration-log.md` is a historical audit
trail; consult it only when a specific decision, commit, or incident needs investigation.

When compaction happens or recovery starts from a compressed summary, re-ingest only:

1. `docs/implementation-roadmap.md`
2. `docs/orchestration-context.md`
3. `docs/orchestration-learnings.md`
4. `docs/active-task-map.md`

Keep the active task map limited to work that still needs attention: active workers,
complete-but-unreviewed branches, corrections, merge cleanup, blockers, and pending decisions.
Update it as the last step before ending an orchestration operation, after it is clear which tasks
remain unresolved. Archive finished tasks by pointing to result logs instead of copying their
details forward.

### Preserve Product Intent

Before accepting work, check that it still fits the core shape:

- `Task` remains the unit of attention.
- Repos, branches, and worktrees remain technical anchors, not the product center.
- Codex credentials stay owned by Codex.
- Codex integration stays behind an adapter boundary.
- The app remains local-first.
- UI work remains usable, scannable, and grounded in real workflows.

If multiple checks fail, stop and course-correct before launching more implementation work.

## Worker Completion Contract

Workers should at minimum report a brief summary of the work they completed. Ask for any additional
details needed for post-work verification in the worker prompt.

## Judgment Cues

- Use fresh worker conversations for independent slices; continue an existing worker when correcting
  or extending its own work.
- Use the lowest reasoning effort that fits the risk: low for mechanical work, medium for normal
  slices, high for architecture/runtime/security, xhigh only for unusually tangled failures or major
  redesign.
- Keep trivial orchestration work in the main thread when it is faster than launching a worker:
  branch/status checks, small doc updates, active-task-map edits, merge notes, and simple follow-up
  prompts.
- Split out admin or review workers only when the work is parallelizable and nontrivial, such as
  messy merge repair, broad verification, or state reconciliation across several branches.
- For UI-heavy slices, decide before launch whether the work needs reusable components, isolated
  review, or a separate usability pass.
- If conceptually separate work can be done independently in parallel, you may launch several
  workers in parallel.
