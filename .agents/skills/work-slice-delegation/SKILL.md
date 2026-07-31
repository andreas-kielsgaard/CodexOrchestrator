---
name: work-slice-delegation
description: Coordinate one focused work slice from an accepted live next-work planner handoff. Use in the planner thread for a single slice or in a planner-owned fork for one parallel slice; build/start the independent orchestration-worker prompt, receive worker completion/blocker notification, and continue the same delegation path through review-before-merge, merge/reconciliation, work-slice reporting, and planner notification.
---

# Work Slice Delegation

## Role

Run as the planner-owned delegation path after `orchestration-next-work-planner` selects executable work. For a single slice this may be the planner thread itself; for parallel work this should be a planner fork dedicated to one slice. Preserve semantic lineage through:

- `sourcePlannerThreadId`
- planner batch id or decision capsule
- root acceptance capsule, only when this is an older root-mediated flow
- planner final output or exact planner delegation handoff payload

Stage exactly one independent worker-root for one work slice, then own coordination across callbacks through review, merge/reconciliation, reporting, and planner notification.

For shared ownership, owner-liveness, work-route, context-routing, and notification concepts, read `../_orchestration-common/concepts.md` when the prompt is unclear.

Do not stage a worker from a root acceptance capsule alone. If the planner handoff payload or planner final output is missing, stop and ask the planner fork for the missing planner output.

Do not implement the slice. Start the worker as an independent `root-orchestration-worker` thread when thread tools are available; otherwise return the complete worker launch prompt, notify the planner route when possible, and clearly mark the delegation as waiting for worker-root startup.

Treat worker startup as an intermediate stage. Complete this delegation only after the slice is reviewed, merged or signed off, reported, and the source planner route is notified, or after the slice is blocked or escalated.

When starting a worker root, apply the shared reasoning-routing and thread-naming concepts. Omit model overrides unless the human explicitly requested a model.

## Inputs

Expect:

- accepted planner decision
- planner decision capsule: what the planner selected, parked, or escalated
- root acceptance capsule, only if supplied by a root-mediated startup flow
- source planner thread id, if known
- planner delegation handoff payload for this slice, or planner final output containing that payload
- slice goal and why it matters now
- project/repo/worktree expectations, including change-target repo and starting cwd
- read-only context repos and no-edit repos
- orchestration home path and repo locator path, if the worker must report into orchestration records
- relevant constraints and boundaries
- files, docs, or references the worker should read
- verification expectations
- reporting and review requirements

If available, create or update a compact `sub-agent-context` relationship record keyed by this delegation actor id for compaction recovery. Do not store the worker prompt, slice task, acceptance criteria, or repo context there.

## Prompt Construction

Write a worker launch prompt that includes all context needed for the worker to act intelligently. Use the planner's delegation handoff payload as the primary source of slice-specific context.

When an older root-mediated flow supplies planner output:

1. Use the supplied planner final output or exact planner handoff payload.
2. Extract the accepted slice's delegation handoff payload when the full planner output was supplied.
3. Confirm it matches the root acceptance capsule.
4. Only then construct the worker prompt.

Do not assume the worker will inherit root or planner context. Do not ask the root to reconstruct worker-facing details from its compact decision capsule.

Include:

- work slice title
- goal and acceptance criteria
- planner justification for creating the slice
- relevant context and references
- explicit non-goals and boundaries
- branch/worktree expectations
- change-target repo and exact starting cwd/worktree
- read-only context repos and what may be inspected there
- no-edit repos or paths
- files or docs to read first
- broad validation-placement clue and deferred-validation owner, if any
- expected result log/report path, if applicable
- orchestration home or locator path for reporting and compaction recovery
- requirement to use `orchestration-worker`
- compact skill context capsule: shared concepts path, relevant downstream skill names, and reload-after-compaction instruction
- review payload requirement for `review-before-merge`
- report-back path to this delegation actor and source planner thread, plus direct human-input route when supplied
- lifecycle-notification requirement: on blocked, complete, or needs-clarification, the worker must send a compact status message to this delegation actor before ending the turn

Exclude:

- broad orchestration history
- unrelated worker reports
- root-only decision debates
- planner reasoning not needed for the slice
- root-carry context that does not affect this worker
- speculative future plans not needed for this slice

Give the worker only the orchestration home or locator information it needs for reporting and recovery. Do not tell the worker to read the full orchestration archive before starting work.

Pass the planner's validation-placement clue without expanding it into a test plan. The worker chooses how to validate. Make clear that deferred validation does not reduce the worker's implementation, deliverables, or local acceptance criteria.

For cross-repo orchestration, make repo routing explicit. A task may write in one repo while inspecting another repo read-only. Example: a dry-run report may be written in `Agent-OS` while inspecting Convivial Medicine read-only. Do not leave that distinction implicit.

## Work Route And Content Dependency

Apply the shared work-route/content-dependency concept. Give the worker a clean route and only make a ref or commit fatal when the planner explicitly made it content-critical.

## Stage Continuation

The worker should start as an independent `root-orchestration-worker` thread using the prompt you produce. Give the worker thread a role/slice title when thread tooling supports it.

Request `thinking: medium` for the worker root by default. Request `thinking: high` when the slice involves subtle architecture, migration logic, cross-repo interpretation, ambiguous validation, or a boundary that could easily be overrun.

Whenever progress depends on another actor, tool, or human and the required request has been delivered, record the exact waiting stage and end the current turn. Resume when a callback or new message supplies actionable input. Coordination ownership persists across turns without active waiting or repeated status messages.

When the worker completes, continue this same delegation actor with `review-before-merge` using the review payload defined by `orchestration-worker`. Do not start a separate review thread for normal flow.

After review:

- if the decision is `merge`, continue this same delegation actor with `merge-accepted-work`
- if the decision is `reconcile`, continue this same delegation actor with `merge-reconciliation`
- if the decision is `re-prompt-worker`, send the correction prompt to the independent worker and keep the delegation lifecycle waiting on worker correction
- if the decision is `sign-off-without-merge`, continue this same delegation actor with `work-slice-reporter`
- if the decision is `human-needed`, ask for the exact decision needed and keep the delegation lifecycle `waiting-on-human`

After `merge-accepted-work` or `merge-reconciliation`, continue this same delegation actor with `work-slice-reporter`. When reporting is complete and the record-root handoff is made or clearly requested, notify the planner fork that the slice is settled, blocked, signed off, or `waiting-on-human`.

If review requests changes, the re-prompt goes back to the independent worker thread.

When the worker reports blocked or needs clarification, forward the compact status to the planner fork. If the issue is attention-only route/ref drift, keep the existing worker route and continue rather than replanning.

## Reasoning Guidance

Use medium reasoning by default. Use high reasoning when the slice boundary is subtle or when bad prompt scope could cause wasted work.

## Output Contract

Return:

- worker launch prompt
- worker thread setup notes, including worker thread id and requested/applied reasoning if started
- source planner thread id and root acceptance reference
- planner output source used: exact handoff payload or supplied planner final output
- required review payload schema
- context intentionally excluded
- sub-agent-context relationship details, if created
- orchestration home or repo locator details passed to the worker
- lifecycle-notification routes passed to the worker
- current stage: worker startup, waiting on worker, review, merge, reconciliation, reporting, planner notification, blocked, or settled
