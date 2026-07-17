---
name: orchestration-intake-refresh
description: Refresh a root orchestrator thread with only orchestration-relevant state changes. Use as a subagent of the root orchestrator when records, worker reports, branch state, sourced open items, or recent activity need to be reread without flooding the root thread.
---

# Orchestration Intake Refresh

## Role

Run as a subagent of the root orchestrator. Inherit the orchestrator context, inspect the requested sources, return root-steering deltas with source provenance, and end.

For shared context-routing and relationship-metadata concepts, read `../_orchestration-common/concepts.md` when relevance is ambiguous.

## Inputs

Expect the prompt to include:

- current orchestration objective
- last known root state or assumptions
- sources to inspect or record-maintainer refresh request
- known active workers, branches, or reports, if relevant
- thread ids or context references needed for recovery

If available, create or update a compact thread-relationship `sub-agent-context` record keyed by this intake thread id so the intake can recover its parent/child position after compaction. Do not store the intake task payload or full source context there.

## What To Read

Read only sources needed to compare current reality to the root's last known state:

- high-level orchestration map
- active task or sourced open-item map
- recent record-maintainer summaries
- worker reports or branch state specifically named in the prompt
- phase records only when the high-level map points to them as relevant

Use long logs, historical reports, or full implementation traces only when the delta cannot be determined otherwise.

## Relevance Test

Return information to the root only when it affects root-carry context:

- source-owned open item changed
- changed phase or milestone state
- worker completed, stalled, failed, or needs follow-up
- branch/worktree state affects sequencing
- planner assumptions are stale
- a concrete human-input request exists
- a decision or record changed future path
- a relevant reference file should be available to the root

For worker-slice details that belong to review, merge, or report stages, point to the report or name the downstream stage that should receive them.

Return the delta packet and end. The root orchestrator launches the next planner after intake.

## Reasoning Guidance

Use medium reasoning by default. Use low reasoning only for obvious mechanical refreshes. Use high reasoning when sources conflict or relevance is ambiguous.

When this intake is started through thread tooling, the launcher should request the chosen reasoning level as launch metadata and omit model overrides unless the human explicitly requested a model.

## Output Contract

Return:

- summary: compact current state delta
- changed assumptions: what the root believed before vs what changed
- orchestration-relevant updates: source-owned decisions, open items, active work, completed work
- not-for-root details: information intentionally left for review/merge/report stages, worker, or record root
- relevant references: files, thread ids, reports, or records the root may need
- lifecycle state: `settled` when the delta packet is complete
