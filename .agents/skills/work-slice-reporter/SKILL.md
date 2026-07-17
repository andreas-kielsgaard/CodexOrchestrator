---
name: work-slice-reporter
description: Produce the completion report for an orchestration work slice after review, merge, reconciliation, or sign-off without merge. Use to gather planner, worker, review, correction, human-input, repository-integration, and merge context into a compact report and to author the prompt for the record-root maintainer.
---

# Work Slice Reporter

## Role

Close the work slice with a durable completion report and a record-maintainer prompt. Gather the owning stages' outputs into a compact source package for record maintenance.

Run this as the reporting stage in the work-slice-delegation thread after `review-before-merge`, `merge-accepted-work`, or `merge-reconciliation`.

Write the completion report under the orchestration home when available, not inside the target repo. Use target repo files only when the user explicitly asks for a repo-local report.

For shared reporting-flow, context-routing, and relationship-metadata concepts, read `../_orchestration-common/concepts.md` when completion routing is unclear.

## Inputs

Expect:

- planner decision and justification
- delegation prompt or summary
- worker completion payload
- review result
- correction loop summary, if any
- merge or reconciliation result, if any
- sign-off reason if not merged
- stage-owned open items, stage decisions, and human responses
- orchestration home path
- repo locator path, if needed to rediscover the orchestration home

If available, update the delegation thread's compact relationship metadata to show the current stage as `reporting`. Do not store the completion report or record-maintainer prompt there.

## Report Contents

Write a report that includes:

- work slice title
- role in the larger project or orchestration scope
- planner decision justifying the slice
- worker thread and delegation thread ids, if known
- branch/worktree and technical scope
- details of how the slice was achieved
- validation performed
- review result and findings
- correction back-and-forth summary
- stage-owned open items, stage decisions, and resolutions
- merge, reconciliation, sign-off, and repository integration outcome
- references to logs, diffs, reports, or records
- record-maintainer source material

Keep raw logs out of the report unless essential. Link or point to them.

## Record-Maintainer Prompt

Author a concise prompt for `orchestration-record-maintainer`. Parent/source route for maintainer: `root-orchestration-record`.

When thread tooling allows, instantiate the maintainer by forking the root record thread and sending the maintainer prompt to that fork. The prompt is reporter-authored; the thread source/parent is the record root. If direct record-root forking is unavailable, send the prompt to the root record thread and ask it to spawn the maintainer.

Include:

- brief slice summary
- path or content of the completion report
- high-level state changes
- source-owned decisions that affect future orchestration
- stage-owned open items and resolutions
- repository integration state from review, merge, or reconciliation
- what should be pruned, linked, or elevated
- planner callback route, when the slice belongs to a planner batch and record settlement should wake that planner
- root orchestrator thread id and intake-wakeup route, when the record change affects root-carry state
- requested maintainer reasoning: medium by default, high if correction, merge, human-input, or pruning history is subtle

## Planner Notification

If a `sourcePlannerThreadId` or planner notification route is available, notify the planner fork when the slice is complete, `waiting-on-tool`, signed off without merge, abandoned, or `waiting-on-human`. Keep the notification brief: slice id/title, outcome, branch/worktree, report path, merge/sign-off state, stage-owned open items, and whether the planner batch can continue.

If record maintenance is also being requested for the slice or batch, tell the planner which record-root/maintainer route has the callback responsibility. The planner waits for the record-side callback after the update settles.

Normal completion and batch-settlement route: planner notification and record-root material. If the planner route is unavailable, put the planner-facing update and desired callback route in the record-maintainer prompt or root-record material. Direct human-input route applies only when a stage explicitly requested a human decision.

## Reasoning Guidance

Use medium reasoning by default. Use high reasoning when the slice involved corrections, human input, or complex merge outcomes.

## Output Contract

Return:

- report path or report content
- concise completion summary
- maintainer prompt for root-orchestration-record
- context intentionally omitted from root
- planner notification sent and/or planner callback route supplied, if applicable
- requested maintainer reasoning
