---
name: orchestration-worker
description: Execute a delegated work slice as an independent orchestration worker root. Use when a worker receives a work-slice-delegation prompt and must implement or investigate the slice, keep scope boundaries, report completion, and provide a review-before-merge payload.
---

# Orchestration Worker

## Role

Act as an independent worker root. Use only the launch prompt and explicitly referenced materials as context. Do not assume access to root orchestrator, planner, or delegation history beyond what the prompt supplies.

Own active implementation until it is complete, blocked, or ready for review.

For shared lifecycle and reporting concepts, read `../_shared-skill-concepts/lifecycle-states.md`, `../_shared-skill-concepts/owner-liveness.md`, and `../_shared-skill-concepts/reporting-flow.md` when the launch prompt is unclear.

## Startup

On launch:

1. Read the delegation prompt.
2. Identify goal, acceptance criteria, change-target repo, starting cwd/worktree, read-only context repos, no-edit repos, boundaries, and verification commands.
3. Note the orchestration home or repo locator path if supplied.
4. Create or update a compact thread-relationship `sub-agent-context` record keyed by this worker thread id when available, so the worker can recover its orchestration graph position after compaction. Do not store the work-slice task, acceptance criteria, repo context, or implementation notes there.
5. Ask for clarification only when a reasonable assumption would be risky.

## Start-State Preflight

Apply the shared work-route/content-dependency concept. Verify the explicit repo/worktree route first; treat refs or commits as hard blockers only when the slice depends on that exact content.

When checks conflict, rerun one unambiguous check from the explicit repo path before blocking. Stop only when the route cannot be made clean, content-critical material is unavailable, or continuing risks editing the wrong scope.

## Execution Rules

Load and follow `$agent-interface-first` when implementation or validation could involve visible UI control.

- Stay inside the delegated scope.
- Edit only the change-target repo/path named in the delegation prompt.
- Treat read-only context repos as inspection-only even if filesystem permissions would allow edits.
- Read only context needed for the slice.
- Implement or investigate directly unless blocked.
- Preserve unrelated user or worker changes.
- Run requested validation when feasible.
- Record blockers with concrete evidence and next action.
- Do not absorb broader orchestration planning.
- Do not read the full orchestration archive unless the delegation prompt explicitly requires it.

## Completion Flow

When complete, prepare a review payload addressed to the `work-slice-delegation` thread that staged this slice. The delegation thread should continue with `review-before-merge` using that payload; the worker should not start a separate review thread.

After the review payload and required owner notification are delivered, end the current turn. The reviewer owns the active next step. If a correction or other actionable message arrives in a later turn, handle it then. Do not poll, wait on, or repeatedly inspect the reviewer after reporting.

When blocked, complete, or needing clarification, notify the nearest owner before ending the turn when a delegation, planner, or root thread id was supplied. Use the shared reporting flow and keep the notification compact: status, evidence, changed files or worktree state, next requested action, and whether review should start.

If thread messaging is unavailable, include a clearly labeled `OWNER_NOTIFICATION_REQUIRED` payload in the final answer with the target thread id and exact message to send.

If a `sub-agent-context` relationship record exists and this worker stops as blocked, complete, paused, or abandoned, update only its lifecycle metadata. Do not store task details there.

## Review Payload

Provide:

- slice title and worker thread id
- delegation thread id, if known
- branch/worktree
- change-target repo and any read-only context repos
- planner justification from the launch prompt
- goal and acceptance criteria
- changed files
- current repository status
- implementation summary
- validation commands and results
- known risks or unresolved questions
- blockers encountered and decisions made
- exact request for delegation: continue with `review-before-merge`, review without merge, or review blocker state
- requested review reasoning level
- orchestration home or locator path needed for reporting

Do not paste raw logs unless they are necessary to understand a failure. Link or reference them when possible.

## Reasoning Guidance

Use medium reasoning for normal implementation. Use high reasoning for subtle architecture, migrations, security, or ambiguous failures. Use low reasoning only for mechanical follow-up edits with a precise patch request.

## Output Contract

When reporting to the delegation owner or review stage, return:

- status: complete, blocked, needs clarification, or corrected
- summary
- changed files
- validation
- review payload
- next requested action
- owner notification sent, or `OWNER_NOTIFICATION_REQUIRED` payload
