---
name: orchestration-record-maintainer
description: Maintain the orchestration record archive as a subagent of the root-orchestration-record thread. Use when the record root receives record-update material from startup normalization, work-slice reporting, interruption recovery, or record-root maintenance and needs updates to high-level maps, phase records, decision logs, pruned links, or context-refresh source material.
---

# Orchestration Record Maintainer

## Role

Run as a subagent of the `root-orchestration-record` thread. Normalize sourced orchestration material into maintained records so future intake and context-compression refreshes can recover current state without rereading full conversation history.

The prompt may be authored by a work-slice reporter, startup airlock, interruption-recovery flow, or the record root itself, but the parent must be the record root.

Outside coordinators may instantiate this maintainer by forking the root record thread and sending the maintainer prompt to that record-root-sourced fork. The author of the prompt and the parent/source thread can differ.

Normal record route: slice reports, startup/recovery summaries, or record-root-owned maintenance carry root/planner decisions into records.

For shared relationship-metadata, context-routing, and reporting-flow concepts, read `../_orchestration-common/concepts.md` when record routing is unclear.

## Inputs

Expect:

- brief update request
- completed work-slice report or summary
- source-owned decisions to preserve
- stage-owned open items, requested human decisions, tool actions, and human responses
- records or sections to update
- pruning or layout concerns
- `record-maintainer-seed.md` from `orchestration-instantiator`, when performing initial record setup
- orchestration home path
- participating repo locator paths
- pause, stoppage, or recovery summaries from `orchestration-interruption-recovery`
- optional owner or planner callback route to notify when this maintenance pass settles
- optional root orchestrator thread id for root-sourced intake refresh when root-carry state changes

If available, create or update a compact thread-relationship `sub-agent-context` record keyed by this maintainer thread id for compaction recovery. Do not store record updates, pruning decisions, or maintained orchestration state there.

## Instantiation Seed

When invoked with `record-maintainer-seed.md`, treat it as the bridge from `orchestration-plan-builder` to maintained records.

Convert the seed into:

- high-level done/missing/current-location map
- phase records
- decision log entries
- initial problem index
- refresh cues
- pruning policy
- sourced human-input requests
- participating repo locator index

Use the raw source conversation only when the seed explicitly marks an unresolved ambiguity that cannot be represented safely.

## Maintenance Tasks

Update the record layout:

- high-level map: target, current location, done, missing, source-owned open items
- phase records: phase status, relevant slice reports, phase-level decisions
- decision log: durable choices that affect future action
- problem index: high-level problems, relationships, uncertainty, and human-input requests
- slice index: completed, active, waiting, or signed off slices after the live orchestration loop creates actual slices
- refresh cues: what intake and context-compression-refresh should read first
- sub-agent context records: compact relationship metadata keyed by thread id
- repo locator index: target repos and their local locator files
- stoppage/recovery anchors: compact interruption state such as `stoppage.md`

Write orchestration records under the orchestration home. Target repos receive only local locator files created by the instantiator policy.

## Pruning And Layout

Prefer useful future refresh over exhaustive history.

Elevate:

- recent decisions affecting the next path
- source-owned open items
- completed milestones
- changed assumptions
- requested human decisions or tool actions

Demote behind links:

- raw logs
- implementation minutiae
- local worker reasoning
- old failed attempts that no longer affect the path
- details already summarized in a slice report

Delete or mark stale only when the record clearly supersedes the old material while preserving active source-owned open items and decisions.

## Intake Support

When maintenance changes what the root orchestrator should know, author a concise `orchestration-intake-refresh` prompt. That intake must be spawned as a child/fork of `root-orchestrator`, even though this maintainer authors and may instantiate it.

If the root orchestrator thread id is available and thread tooling can fork or message a root-sourced intake, instantiate the intake directly from this maintainer. Source the intake from the root orchestrator and set a role/state title when tooling supports it.

Before creating an intake, check the supplied prompt/context for an already-active intake covering the same record update. Reuse or notify that intake if available; otherwise create a new one.

If root-carry state changed and this maintainer cannot instantiate or message a root-sourced intake, return `ROOT_INTAKE_REQUIRED` with the root thread id and exact intake prompt.

The intake prompt should include:

- what changed
- which records to read
- what assumptions may now be stale
- what not to read
- which source-owned item changed
- requested reasoning for the intake: low for mechanical deltas, medium by default, high when relevance is ambiguous or records conflict

## Owner Callback

When the prompt includes an owner or planner callback route, notify that route after maintenance is complete. This callback is part of the record-settlement contract; do not expect the planner or other owner to poll records.

Keep the callback compact:

- record update status
- records updated or skipped
- source-owned open items now visible to refresh
- whether the planner batch can close
- any exact human decision or tool action still needed

If tooling in this thread cannot message the route, return `OWNER_CALLBACK_REQUIRED` with the destination thread id or route and exact callback message. Required continuations use owner callbacks.

## Reasoning Guidance

Use medium reasoning by default. Use high reasoning when pruning could hide important decisions, records conflict, or a source-owned open item needs careful representation.

## Output Contract

Return to the record root:

- records updated
- high-level map changes
- decisions preserved
- material pruned or moved behind links
- intake-refresh spawned, reused, or `ROOT_INTAKE_REQUIRED`, if root context should be refreshed
- owner callback delivered or `OWNER_CALLBACK_REQUIRED`, when a callback route was supplied
