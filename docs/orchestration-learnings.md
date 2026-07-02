# Orchestration Learnings

This file captures practical lessons for running the control-room plus worker-chat process. Treat
"Guardrails" as authoritative. Treat "Skill Cues" and "Incident Cues" as prompts for better
judgment, not extra procedure.

## Guardrails

### Worker Integration Unit

Implementation workers must produce an integration unit the orchestrator can review:

- dedicated task branch or clearly named existing branch
- committed completed slice unless blocked or exploratory
- result log under `docs/task-logs/`
- verification summary
- exact `git status --short --branch`
- specific review notes

If a Codex worktree starts detached, the worker should create the assigned branch before editing.

### Report Back Is Required

Every worker launch prompt needs a dedicated "Report back" section. The worker must send the
completion report to the orchestration thread when done. If cross-posting is unavailable, the worker
must say so in its own final message and still include the full report.

### Review Before Merge

The orchestrator reviews worker branches before merging, runs independent verification, and records
corrections or merge decisions. Dependent work should wait until upstream branches are accepted or
explicitly rejected.

### Context Recovery

Recovery is active-task-map based. On compaction or compressed-summary resume, read the current
orchestration docs and `docs/active-task-map.md`; do not replay old handoff chains or scan the
orchestration log by default.

Keep the active task map current and small. Remove tasks once they are reviewed, merged or rejected,
verified, logged, and Git-cleaned. Update it as the last step before ending an orchestration
operation so it reflects unresolved work rather than momentary in-progress state.

### Visible Trace

Keep worker/admin chats visible unless the user explicitly asks to archive them. Cleanup Git state;
do not hide the conversation trail.

## Skill Cues

### Prompt Shape

Good worker prompts are compact and explicit:

- what to build
- why it matters now
- what to read
- what not to touch
- branch/worktree expectations
- verification commands
- result-log path
- report-back instruction

Avoid copying hidden context or long historical summaries into worker prompts.

### Fresh vs Continued Workers

Start fresh when the next slice is independent or old details would bias the work. Continue a worker
when the task is a correction, extension, or debug pass on that worker's own branch.

### Keep Going After Clean Merges

A clean checkpoint is a decision point, not a stopping point. Continue to the next smallest useful
slice unless there is a blocker, product question, failed review, unclear next action, or recovery
checkpoint to update.

### UI Slices

For UI-heavy work, protect usability early. Ask for states, affordances, density, and user workflow
clarity. Consider a separate usability-review worker when a screen becomes substantial.

### Admin Work

If merge repair, branch cleanup, or state reconciliation becomes nontrivial, split it into a focused
admin task instead of burying it inside implementation work.

### Windows Worktree Cleanup

Check registration and branch state before cleanup. If Windows keeps a physical folder locked after
`git worktree remove`, log it and leave it alone rather than force-deleting.

## Incident Cues

- Worker 001 showed that branch/commit requirements must be explicit.
- Early cleanup showed that Windows may leave locked physical worktree folders after Git state is
  clean.
- Worker chats were once archived too aggressively; visibility matters for traceability.
- Worker 005 showed that prompts must distinguish the main checkout from the assigned worktree.
- Worker 006 showed that orchestration should not stop merely because a merge checkpoint is clean.
- Worker 015 showed that report template alone is not enough; the prompt must explicitly say to
  report back.
- Later compaction handling showed that handoff chains and the orchestration log were too heavy for
  recovery; `docs/active-task-map.md` is the current recovery mechanism.
