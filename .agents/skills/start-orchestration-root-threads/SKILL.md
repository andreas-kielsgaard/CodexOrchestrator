---
name: start-orchestration-root-threads
description: Start the live root orchestration threads from an instantiated orchestration package. Use after orchestration-instantiator has created root seed files and before live orchestration begins, especially when converting verbose startup material into clean root-orchestrator and root-orchestration-record threads.
---

# Start Orchestration Root Threads

## Role

Act as the startup airlock between orchestration seeding and live orchestration. Convert the verbose instantiated package into clear root-thread launch prompts, create the root record and root orchestrator threads when explicitly requested and tools are available, write thread ids back into the orchestration package, and clean consumed startup scaffolding after launch.

Do not plan project work, launch workers, or normalize records yourself. Your job is to start the two root threads cleanly, preserve their orientation, and leave the orchestration folder in a live-run shape.

For shared ownership, prompt-context, and relationship-metadata concepts, read `../_orchestration-common/concepts.md` when startup semantics are unclear.

## Inputs

Expect:

- orchestration home path
- `root-record-start.md`
- `root-orchestrator-start.md`
- `record-seed.md`
- `record-maintainer-seed.md`
- `orchestration-plan.md`
- `problem-map.md`
- `participating-repos.md`
- `repo-locators.md`
- `sub-agent-context.md`
- user instruction about whether to create threads or only prepare prompts

If the orchestration home is unknown, use the repo locator file only to rediscover it.

## Interpretation Semantics

Label the seed files for the new roots before launch:

- `root-record-start.md`: operational launch prompt for the record root.
- `root-orchestrator-start.md`: operational launch prompt for the orchestrator root.
- `orchestration-plan.md`: approved strategic context, not a work order.
- `problem-map.md`: high-level problem architecture for later control decisions, not executable slices.
- `record-seed.md`: initial record facts.
- `record-maintainer-seed.md`: maintainer-ready material the record root should pass to `orchestration-record-maintainer`.
- `participating-repos.md`: repo roles and routing metadata.
- `repo-locators.md`: locator inventory.
- `sub-agent-context.md`: relationship recovery metadata, not the orchestration ledger or a task archive.

The live roots should receive a concise startup capsule plus references to the verbose files. They should not be asked to ingest every seed file as equally important active context, and they should not start with planning-adjacent wording that narrows the run before the control loop has verified current state.

## Startup Capsule

For each root thread, prepare a short capsule:

- orchestration title and slug
- orchestration home
- role boundary
- authoritative startup files
- current state
- known blockers or unknowns
- first action
- what not to do yet

Use the verbose files as reference material behind the capsule.

Prefer operational terms in startup prompts:

- use "startup context" instead of "accepted plan package"
- use "first control action" instead of "first planning action"
- use "problem map" as context, not as an execution queue
- mention `orchestration-next-work-planner` only as the later skill for choosing executable next work from verified current state

## Thread Start Order

If thread tools are available and the user asked to create threads:

1. Start the `root-orchestration-record` thread first with the record capsule and `root-record-start.md`, requesting `thinking: medium` unless startup records conflict.
2. Capture the record thread id.
3. Give the record root a role-specific title when thread tooling supports it.
4. Update `sub-agent-context.md`, `root-orchestrator-start.md`, and repo locator files with the record thread id.
5. Start the `root-orchestrator` thread with the orchestrator capsule and updated `root-orchestrator-start.md`, requesting `thinking: medium` by default or `thinking: high` when launch state is stale, cross-repo routing is unclear, or startup cleanup depends on interpretation.
6. Capture the orchestrator thread id.
7. Give the orchestrator root a role-specific title when thread tooling supports it.
8. Update `sub-agent-context.md` and repo locator files with the orchestrator thread id.
9. Confirm the record root has received or normalized the startup seed material, or record that this cleanup is pending.
10. Clean consumed startup scaffolding from the active orchestration folder.
11. Return both thread ids and the next user action.

If thread tools are unavailable or the user asked for prompts only, produce the two final startup prompts and clearly state what should be pasted or used to start each root.

## Startup Cleanup

Treat startup seed files as launch scaffolding. After the root threads have started and the record root has absorbed the seed material, remove consumed scaffolding from the active orchestration folder.

Default consumed startup files:

- `root-record-start.md`
- `root-orchestrator-start.md`
- `record-seed.md`
- `record-maintainer-seed.md`

Keep active:

- `orchestration-plan.md`
- `problem-map.md`
- `participating-repos.md`
- `repo-locators.md`
- `sub-agent-context.md`
- maintained record files created by the record root or record maintainer

Before deleting, verify that either:

- the root record thread has confirmed ingestion or normalization of the seed material, or
- the same information now exists in maintained records under the orchestration home.

If confirmation is unavailable, move the consumed files into a clearly temporary startup cleanup state only if the user asked for cleanup anyway; otherwise leave them and report "startup cleanup pending." Do not let seed files continue to act as living records.

## Root Record Launch

Tell the record root:

- use `orchestration-record-root`
- own maintained records, not project direction
- treat the orchestration home as record source of truth
- start by using `record-maintainer-seed.md` to spawn `orchestration-record-maintainer` if normalization is needed
- confirm when startup seed material has been absorbed so the airlock can remove scaffolding
- return a compact startup/record summary and any intake prompt for the orchestrator

## Root Orchestrator Launch

Tell the orchestrator root:

- use `orchestration-root`
- own direction and next-work coordination
- treat the approved plan and problem map as strategic context, not as executable slices
- confirm missing repo paths and current repo state before choosing executable work
- build a compact skill context capsule before creating planner forks
- use `orchestration-intake-refresh` if state may have changed
- use `orchestration-next-work-planner` to choose executable next work from current reality
- do not launch worker roots until a next-work plan is accepted

## Reasoning Guidance

Use medium reasoning by default. Use high reasoning when the seed files conflict, repo locator paths are missing or inconsistent, thread ids need to be patched into multiple places, or the distinction between seed context and live orchestration context is unclear.

When thread tools support it, set reasoning as `thinking` launch metadata. Do not set model unless the human explicitly requested one. If creating prompts only, state the requested reasoning level in each prompt.

## Output Contract

Return:

- startup mode: created threads or prompts only
- orchestration home
- interpretation summary
- root record thread id or prompt
- root orchestrator thread id or prompt
- files updated
- startup scaffolding removed or cleanup still pending
- unresolved startup issues
- immediate next action
- requested/applied reasoning for each root thread
