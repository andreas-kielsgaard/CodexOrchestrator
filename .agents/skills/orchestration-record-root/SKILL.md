---
name: orchestration-record-root
description: Run the root orchestration record thread that owns maintained project memory for orchestration. Use when coordinating record-maintainer subagents, preserving high-level done/missing maps, pruning historical detail, and supporting context refresh without making the orchestrator thread carry archival work.
---

# Orchestration Record Root

## Role

Act as the long-lived record thread for an orchestration run. Own the maintained memory substrate in the orchestration home, not project direction. The root orchestrator decides what to do next; this thread keeps records useful, pruned, and refreshable, and notifies requesting owners when record maintenance settles.

Do not make this thread a child of the root orchestrator, a planner, a worker, or a reporter. It is its own root.

For shared ownership, relationship-metadata, and reporting-flow concepts, read `../_orchestration-common/concepts.md` when record routing is unclear.

## Startup From Instantiation

When started from an `orchestration-instantiator` package:

1. Read the root-record start prompt.
2. Note the orchestration home path and participating repo locator files.
3. Load the approved plan summary and `record-seed.md`.
4. Load `record-maintainer-seed.md` when provided.
5. Spawn `orchestration-record-maintainer` as a child of this record root if the seed needs to be normalized into maintained records.
6. Confirm when the startup seed material has been absorbed into maintained records so `start-orchestration-root-threads` can remove the consumed scaffolding from the active orchestration folder.
7. Return a compact record-root startup summary and any `orchestration-intake-refresh` prompt that the root orchestrator should run.

Record normalization belongs to this record root and its maintainer children.

Treat `root-record-start.md`, `root-orchestrator-start.md`, `record-seed.md`, and `record-maintainer-seed.md` as launch scaffolding, not ongoing records. Once their information is normalized into maintained records, they should not remain the source the system keeps rereading.

## Responsibilities

Maintain records that allow later agents to recover:

- the high-level "what is done" overview
- the high-level "what is still missing" overview
- the current phase or project location
- recent decisions that affect the path forward
- phase or slice references pointing to detailed reports
- sourced open items and human-input requests
- pruned links to worker reports, review results, and merge outcomes
- participating repo locator paths

Prefer maps and pointers over copied detail.

## Subagent Use

Spawn `orchestration-record-maintainer` as a subagent of this record root when a work-slice reporter, startup airlock, interruption-recovery flow, or this record root supplies a record-update prompt.

Also spawn it during startup when `orchestration-instantiator` provides `record-maintainer-seed.md`.

Apply the shared reasoning-routing and thread-naming concepts when spawning maintainers: request `thinking: medium` by default and `thinking: high` when pruning, conflicts, or open-item representation could affect future orchestration.

The work-slice reporter may author the maintainer prompt. Maintainer parent/source route: this record root. If an outside coordinator has thread-fork tooling, it may fork this record root directly and send the reporter-authored maintainer prompt to that fork.

If a record-update prompt is sent to this root instead of to a record-root-sourced maintainer, spawn the maintainer immediately when tooling is available. Do not handle nontrivial record updates inline unless no subagent/fork path is available.

Preserve any callback route included in the record-update prompt, especially planner callback routes for batch settlement. Pass the route to the maintainer. After the maintainer completes, the maintainer must notify that route with a compact record-settled payload, or return `OWNER_CALLBACK_REQUIRED` if it cannot. Do not make planners poll records for completion.

If the record-update prompt includes the root orchestrator thread id or root-intake wakeup route, pass it to the maintainer. Do not relay ordinary root-intake handoffs yourself; the maintainer should instantiate the root-sourced intake directly or return `ROOT_INTAKE_REQUIRED`.

After a maintainer child completes, trust the maintainer for record editing. Do not reperform its archive work or broadly reread the same sources. If a final summary needs grounding, do at most a narrow spot check of the specific files the maintainer reports as changed. Do not perform root-intake handoff from this record root.

Ordinary control-flow route for root/planner decisions into records: normal slice reports, startup/recovery summaries, or a record-root-owned maintenance pass.

When `orchestration-interruption-recovery` reports a pause or resume event, preserve the compact stoppage/recovery summary in the orchestration home. Do not own pause/resume coordination; the root orchestrator owns that through `orchestration-interruption-recovery`.

## Record Shape

Preserve a layered record layout:

- high-level map: target, current location, done, missing, sourced open items
- phase records: phase goals, status, decisions, links to slice reports
- slice reports: planner justification, worker summary, review, merge, sourced open items
- decision log: accepted decisions and reversals that affect future work
- sub-agent context records: compact relationship metadata for compaction recovery only
- stoppage or recovery records: compact pause/resume anchors such as `stoppage.md`

Do not turn sub-agent context records into a second orchestration ledger.

## Pruning Rules

Optimize records for future refresh, not historical completeness.

Keep:

- recent decisions that affect the next path
- current target and current location
- sourced open items
- links to detailed reports
- durable conclusions from reviews and merges

Prune or move behind references:

- raw logs
- implementation minutiae
- old attempts that no longer affect choices
- worker-local reasoning
- details already represented by a higher-level conclusion

## Output Contract

When responding to a record-maintainer update, return:

- records updated
- high-level state changes
- sourced decisions or open items now visible to refresh
- stale or pruned material
- recommended next refresh, if any
- maintainer callback/intake status, including `OWNER_CALLBACK_REQUIRED` or `ROOT_INTAKE_REQUIRED` if the maintainer could not deliver
- requested/applied maintainer reasoning when this root launched the maintainer
