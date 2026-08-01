# Ad-hoc unattended liveness

## Observation

The user authorized the current Codex development workflow to continue unattended and requested a two-hour recovery task.

- The Batch 3 worker sent its completion callback to planner task `019fb99b-46ad-71b2-b18a-6bfdd995e654` at 2026-07-31 22:43 Europe/Copenhagen.
- The callback was delivered as an unread task message, but no planner turn started until the root explicitly continued it at 2026-08-01 08:17.
- The worker's messaging call returned no receiver-activation evidence, although its final response said the work was reported.
- The retained watchdog run could not read the task index or root task, inspected clean worktrees instead, recorded liveness as unverified, and steered no task. It did not reach the root or Skill Maintainer. Morning deletion and archiving ended that liveness route; they do not negate its earlier existence or authorize replacement.

## Reader and ownership

The observed readers are `orchestration-root`, `orchestration-next-work-planner`, and `orchestration-worker` in the current ad-hoc Codex development workflow. Their own role text says they are not the future product Epic Runner, Work Slice Planner, or Work Unit Implementer. The future product harness therefore remains a separate design boundary.

## Theory

The ad-hoc skills treated proactive notification as a sufficient continuation boundary. The harness can deliver a task message without activating the receiving task, so the planner became dormant despite having actionable input. The root knew deterministic task wake was unsupported, but its skill did not translate that limitation into an unattended liveness action. Relevant root and planner references also pointed to the removed `_orchestration-common/concepts.md`, so the current shared liveness wording was not reliably discoverable.

The watchdog prompt correctly avoided polling and work-slice duplication, but it had no explicit classification for delivered-but-unactivated callbacks and no task-state fallback beyond Git evidence. The root skill also lacked a singleton identity and cancellation boundary for scheduled control work, allowing later turns to create duplicates or recreate a user-removed task. Clean worktrees could not prove that an ownership chain was advancing.

## Revision concept and evaluation

Use one narrow distinction throughout the ad-hoc flow: callback delivery is not receiver activation. The sender reports both facts and ends its turn without polling. One harness-evidenced liveness task per orchestration/root-planner route owns compact recovery and starts the existing idle owner from the exact recorded continuation. Unverifiable task state cannot authorize another task. User cancellation, removal, or archiving ends the route until explicit renewed authorization. This keeps workers quiet, avoids repeated parent ingestion, and prevents duplicate work or recovery tasks.

The revision is likely to help because it matches the observed harness behavior and assigns each agent only an actionable responsibility. It does not ask children to wait, poll, or keep themselves available. The remaining risk is harness availability: wording cannot start a task when task-state or messaging services are unavailable.

## Applied revision

The external ad-hoc skills were revised through the general skill-authoring path:

- `_shared-skill-concepts/owner-liveness.md` separates message delivery from task activation and gives recovery to the unattended-liveness owner.
- `orchestration-worker` reports delivery and activation evidence separately, then yields.
- `orchestration-next-work-planner` preserves the exact dormant continuation for recovery without rereading worker history.
- `orchestration-root` reuses one harness-evidenced liveness task for an unattended route, treats task-state failure as unverified rather than creation authority, and requires renewed user authorization after cancellation, removal, or archiving.
- Broken shared-concept references in the root and planner were replaced with the current shared paths.

No future product-role skill was changed. Before product skills rely on callbacks, the Codex Orchestrator harness should define durable callback enqueue, receiver activation, idempotent retry, and observable lifecycle evidence. A scheduled checker should be recovery, not the primary execution loop.
