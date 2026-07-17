---
name: context-compression-refresh
description: Refresh a root orchestrator thread after context compression without performing handoff. Use immediately after compaction or suspected context loss to restore current objective, location, recent decisions, blockers, and next actions from maintained orchestration records.
---

# Context Compression Refresh

## Role

Run in the root orchestrator after context compression or suspected context loss. Restore the orchestration working set from maintained records.

Do not perform handoff. Do not plan a new slice unless the refreshed state makes the next action obvious and low-risk. Prefer to recommend `orchestration-intake-refresh` or `orchestration-next-work-planner` when more reasoning is needed.

First rediscover the orchestration home. Prefer the current prompt or thread context. If missing and the thread is inside a participating repo, read only the gitignored locator file such as `.codex-orchestrator/orchestration-link.json` to find the orchestration home.

For shared context-routing and relationship-metadata concepts, read `../_orchestration-common/concepts.md` when deciding what belongs back in root context.

## What Matters Most

Prioritize:

- current target
- where the orchestration is now
- recent decisions that affect the path forward
- active blockers and human-intervention needs
- active stoppage or resume state
- active workers, branches, or delegation threads
- next gate or action
- records the root should have available
- skill context capsule needed before spawning the next planner/delegator/worker or continuing review/merge/report stages

Deprioritize:

- details of all past decisions
- old implementation minutiae
- raw worker logs
- historical attempts that no longer affect the path

Compression loses context. Accept that and restore what the orchestrator needs now.

## Source Order

Read in this order when available:

1. repo locator file, only if the orchestration home is unknown
2. high-level orchestration map from the orchestration home
3. active blocker/task map
4. stoppage.md, if present
5. recent decision log entries
6. current phase record
7. relevant recent slice reports
8. thread-relationship metadata from `sub-agent-context`, only if root/child thread links or lifecycle state are needed

Do not walk the full archive unless the high-level map is missing or contradictory.

## Reasoning Guidance

Use low reasoning for straightforward refresh. Use medium when records are stale or conflicting. Do not use high reasoning to re-plan inside this skill; call planner instead.

## Output Contract

Return:

- refreshed objective
- current location
- recent path-shaping decisions
- active blockers or human needs
- stoppage or resume state, if any
- active work and thread references
- skill context reingested or refresh needed
- next recommended orchestration action
- records read
- records intentionally skipped
- orchestration home rediscovered or used
