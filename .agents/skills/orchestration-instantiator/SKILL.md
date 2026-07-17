---
name: orchestration-instantiator
description: Instantiate an approved orchestration-ready plan into an orchestration-owned directory outside target workspaces, with durable strategic context files, temporary root startup scaffolding, and gitignored repo locator files. Use after orchestration-plan-builder output is approved and before starting the root orchestrator and root orchestration record threads.
---

# Orchestration Instantiator

## Role

Turn an approved orchestration-ready plan into a concrete orchestration startup package.

Create the orchestration startup package, including durable strategic context, temporary startup seed material, and start prompts for the root orchestrator and root record thread. Create threads only when the user explicitly asks for thread creation and the appropriate thread tools are available.

Do not implement work slices. Do not derive executable work slices from the high-level plan. Do not start independent worker roots unless a later live `orchestration-next-work-planner` pass has accepted concrete executable work and the user explicitly asks for immediate launch.

After creating the startup package, use `start-orchestration-root-threads` for the transition from seed material to live root threads and for cleanup of consumed startup scaffolding.

For shared startup, relationship-metadata, and prompt-context concepts, read `../_orchestration-common/concepts.md` when package semantics are unclear.

## Inputs

Expect:

- approved orchestration-ready plan
- `orchestrationPlanDraft` JSON object when produced by `orchestration-plan-builder`
- participating repositories or workspaces
- orchestration home base path, if the user or product supplies one
- preferred plan slug or generated slug
- desired thread creation behavior: prompts only, create threads, or create files only
- any project-specific record conventions
- record-maintainer seed material from `orchestration-plan-builder`

If the input is still raw, use `orchestration-plan-builder` first.

## Intelligence And Reasoning

Use medium reasoning by default. Use high reasoning when deciding cross-repo file placement, when the approved plan is internally inconsistent, or when root-thread startup would be ambiguous.

When this skill creates root threads directly, request reasoning as launch metadata according to the root thread role: record root medium by default, orchestrator root medium by default, and high for either when startup state is conflicted or ambiguous. Do not set model unless the human explicitly requested one.

Reason about startup integrity:

- which context must be delivered directly in prompts
- which details belong in durable strategic context files
- which seed files are temporary startup scaffolding
- which thread-relationship records exist only for compaction recovery
- how to keep root orchestrator and root record responsibilities separate
- what must not leak into independent worker roots

## Orchestration Home

Create the orchestration package outside the repositories being orchestrated. Do not place orchestration-owned records under a target repo's tracked docs unless the user explicitly asks for a repo-local artifact.

Default current path:

```text
~/.codex/orchestrations/<plan-slug>/
```

On Windows this normally resolves under the user's profile, for example:

```text
C:\Users\<user>\.codex\orchestrations\<plan-slug>\
```

In future Codex Orchestrator product flows, use the product-owned orchestration data directory instead.

## Startup Package

Create the package under the orchestration home:

```text
<orchestration-home>/
```

Recommended files:

```text
orchestration-plan.md
orchestration-plan.json
orchestration-live-state.json
orchestration-blockers.json
root-orchestrator-start.md
root-record-start.md
record-seed.md
record-maintainer-seed.md
record-seed.json
problem-map.md
sub-agent-context.md
participating-repos.md
repo-locators.md
```

Treat `root-orchestrator-start.md`, `root-record-start.md`, `record-seed.md`, and `record-maintainer-seed.md` as startup scaffolding. They should exist before root launch and be consumed by `start-orchestration-root-threads`. After the live roots start and the record root confirms ingestion or normalization, the startup airlock should remove these seed files from the active orchestration folder.

Keep `sub-agent-context.md` compact. It follows the shared relationship-metadata concept and is not the main orchestration ledger or task archive.

## Required File Contents

`orchestration-plan.md` should contain the approved plan: objective, current state, accepted decisions, problem architecture, possible phase boundaries, validation gates, non-goals, and human-intervention gates.

`orchestration-plan.json` should contain the product-supported plan structure. Preserve the builder's semantic plan while normalizing stable ids, nested `planRoot.children`, repo routing, dependencies, decision gates, validation concerns, and status fields. This is the file the product overview should use to show where the project sits in the proposed problem structure.

Use this top-level shape:

```json
{
  "schemaVersion": 1,
  "orchestrationId": "<plan-slug>",
  "title": "<title>",
  "objective": "<objective>",
  "scope": {
    "changeTargets": [],
    "readOnlyContext": [],
    "outOfScope": []
  },
  "planRoot": {
    "id": "plan-root",
    "title": "<overall problem>",
    "kind": "objective",
    "summary": "<what this node solves>",
    "status": "proposed",
    "repoRouting": [],
    "dependencies": [],
    "decisionGates": [],
    "validationConcerns": [],
    "children": []
  },
  "plannerAuthority": {
    "mayModifyPlanNodes": true,
    "mayCreateSubNodes": true,
    "mayAttachWorkSlices": true
  }
}
```

Nested `planRoot` nodes represent problems, stages, and sub-stages. They are not initial work slices. The live planner can update them as the product records actual planning, execution, review, merge, report, and record-maintenance outcomes.

`orchestration-live-state.json` should contain the current product projection for UI and routing: current location, active lifecycle nodes, completed plan nodes, active blockers, planner turns, work slices, and record updates. It may start sparse during instantiation, but it should reference `orchestration-plan.json` node ids.

`orchestration-blockers.json` should contain product-addressable blockers from the approved plan. Each blocker should have a stable id, title, state, severity, summary, detail, resolution question, next-planner context, associated plan node ids, associated work slice ids when known, creator role, timestamps, and a nullable resolution. Use this file for user decisions that the product can present directly and later feed into planner prompts.

`record-seed.json` should contain the structured record-maintainer seed corresponding to `record-seed.md`: high-level map, phase/problem index seed, decision log seed, refresh cues, pruning guidance, and human gates.

Write structured JSON files as deterministic artifacts and validate them with a JSON parser before reporting success. In the product implementation, the controller should own these writes directly from structured stage output instead of relying on a long prose turn to materialize every file.

`root-orchestrator-start.md` should contain a launch prompt that invokes `orchestration-root`, names the orchestration, points to the record root, summarizes the current state, and says what the first control action should be.

`root-record-start.md` should contain a launch prompt that invokes `orchestration-record-root`, names the orchestration, describes the record layout, and seeds the high-level done/missing/current-location map.

`record-seed.md` should contain the initial high-level map, phase index, decision log seed, active blockers, and refresh cues.

`record-maintainer-seed.md` should contain the first maintainer-ready update package: high-level map seed, phase/problem record seed, decision log seed, problem index seed, refresh cues, pruning guidance, and human-intervention gates. It should be written so the root record thread can spawn `orchestration-record-maintainer` with it without rereading the raw source input.

`problem-map.md` should list the high-level problems to solve, their relationships, dependencies, uncertainty, likely gates, and validation concerns. It should not contain predetermined worker slices or branch names.

Do not create `work-slice-index.md` during initial instantiation unless the approved plan explicitly includes already-accepted executable slices. Normal operation should let the live root orchestrator and `orchestration-next-work-planner` create actual work slices later.

`sub-agent-context.md` should follow the shared relationship-metadata concept.

`participating-repos.md` should list each repo/workspace touched by the orchestration, its role, expected branch/worktree policy, and whether a local locator file was created. Distinguish change-target repos from read-only context repos and out-of-scope repos.

`repo-locators.md` should list every repo-local locator file path and what it points to.

## Repo Locator Files

For each participating Git repo that will host changes or repeated orchestration work, create or propose a gitignored locator file:

```text
<repo>/.codex-orchestrator/orchestration-link.json
```

The file should contain only rediscovery metadata, such as:

```json
{
  "orchestrationId": "<plan-slug>",
  "orchestrationHome": "<absolute-path>",
  "planPath": "<absolute-path-to-orchestration-plan.md>",
  "rootRecordThreadId": "<known-or-empty>",
  "rootOrchestratorThreadId": "<known-or-empty>",
  "updatedAt": "<iso-timestamp>"
}
```

For read-only context repos, create a locator only when repeated inspection or recovery from inside that repo is expected. Ensure the locator is gitignored. Prefer adding `.codex-orchestrator/` to `.git/info/exclude` for local-only persistence. If the repo already has a local ignored area, use that existing convention. Do not commit locator files unless the user explicitly changes the policy.

## Thread Creation

If thread tools are available and the user explicitly asked to create threads:

1. Create the `root-orchestration-record` thread first using `root-record-start.md`, requesting the selected reasoning level.
2. Capture its thread id.
3. Ask or instruct the record root to use `record-maintainer-seed.md` to spawn its first `orchestration-record-maintainer` child when record normalization is needed.
4. Patch or update `root-orchestrator-start.md` and `sub-agent-context.md` with the record thread id.
5. Create the `root-orchestrator` thread using `root-orchestrator-start.md`, requesting the selected reasoning level.
6. Capture its thread id.
7. Patch `sub-agent-context.md` and repo locator files with the orchestrator thread id.

If thread tools are unavailable or creation was not explicitly requested, output the prompts and file paths instead.

## Root Startup Prompts

The orchestrator start prompt must instruct the root to:

- use `orchestration-root`
- treat the plan package as accepted startup context
- know the orchestration home path and repo locator file paths
- keep raw strategic input out of the root unless it affects current decisions
- use `orchestration-intake-refresh` only when state may have changed
- use `orchestration-next-work-planner` to evaluate the problem map and choose the next executable slice from current reality
- use `work-slice-delegation` after accepting planner output

The start prompts should include interpretation semantics: which files are operational launch prompts, which are strategic references, which are temporary startup seeds, and which are thread-relationship recovery metadata. The root threads should receive a concise startup capsule plus references to verbose files, not an undifferentiated context dump.

Avoid planning-adjacent startup framing. Use "startup context," "first control action," and "verified current state." Mention `orchestration-next-work-planner` as the later skill that chooses executable work only after startup checks and intake.

The record start prompt must instruct the record root to:

- use `orchestration-record-root`
- own records, not project direction
- treat the orchestration home as the record source of truth
- preserve high-level done/missing/current-location state
- load `record-maintainer-seed.md` as the first maintainer-ready material package
- spawn `orchestration-record-maintainer` only from the record root
- use the maintainer seed to establish high-level map, phase/problem records, decision log, problem index, refresh cues, and pruning policy
- support later `context-compression-refresh`
- confirm when startup seed material has been absorbed so `start-orchestration-root-threads` can remove startup scaffolding

## Output Contract

Return:

- files created or proposed
- orchestration home
- structured plan path: `orchestration-plan.json`
- live state projection path: `orchestration-live-state.json`
- product blocker path: `orchestration-blockers.json`
- structured record seed path: `record-seed.json`
- repo locator files created or proposed
- root record prompt path
- root orchestrator prompt path
- record-maintainer seed path
- thread ids if created
- requested/applied reasoning if threads were created
- instantiation assumptions
- immediate next action
- any human action needed before orchestration starts
- next step: use `start-orchestration-root-threads`
