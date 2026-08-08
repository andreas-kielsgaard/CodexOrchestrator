---
name: start-plan-slice
description: Start one separately addressable Plan Slice conversation for an ad-hoc Codex Initiative. Use from the Overall Plan conversation after the next bounded slice is selected and task creation is authorized.
---

# Start Plan Slice

Create one Plan Slice conversation for one bounded movement in the Overall Plan.

## Prepare the handoff

Supply the slice objective, why it is next, current evidence, accepted decisions, constraints, authority, completion or re-evaluation condition, relevant sources, repository or worktree route, and the Overall Plan conversation's callback id.

Give broad clues about where validation may belong without choosing the new conversation's commands, tests, checks, or method. A later integration check does not defer the slice's implementation, deliverables, or local acceptance.

## Start the conversation

Use the host harness to create a separate top-level task with the model and reasoning profile selected by the Overall Plan role, normally Sol with high reasoning. Prompt it to use `run-plan-slice`. A collaboration subagent is not the Plan Slice conversation.

Confirm that no active conversation already owns the same slice. Record the task id and applied profile evidenced by the harness, and send the complete handoff once. After successful delivery, let the new conversation plan and coordinate the slice; do not poll or repeatedly ingest routine progress.

If task creation, profile application, or message delivery is not evidenced, report that boundary without claiming it occurred.
