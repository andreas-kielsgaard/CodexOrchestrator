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

### Parallelize The Nondependent Path

Review-before-merge protects integration quality, but it does not require unrelated work to sit
idle. When a completed branch is being reviewed, launch or continue an independent slice if its
inputs are already stable. Keep dependent slices waiting until the upstream branch is accepted,
rejected, or explicitly declared safe to build against.

### UI Slices

For UI-heavy work, protect usability early. Ask for states, affordances, density, and user workflow
clarity. Consider a separate usability-review worker when a screen becomes substantial.

### Admin Work

If merge repair, branch cleanup, or state reconciliation becomes nontrivial, split it into a focused
admin task instead of burying it inside implementation work.

Keep small admin moves in the orchestration thread: checking status, staging a known doc update,
recording a merge result, or sending a concise follow-up. Launching a worker for those cases adds
more overhead than it saves.
