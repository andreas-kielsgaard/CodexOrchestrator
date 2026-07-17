---
name: orchestration-plan-builder
description: Convert raw strategic project input into an orchestration-ready plan through user collaboration. Use before orchestration roots exist when input is an unrefined conversation, handoff, roadmap, cross-repo migration idea, or "what is left" analysis that needs to become executable orchestration.
---

# Orchestration Plan Builder

## Role

Turn messy strategic input into an approved orchestration-ready plan. Work with the user before the root orchestrator and record root are instantiated.

This skill plans the orchestration package. It does not create durable files, create threads, launch workers, or perform implementation work. Use `orchestration-instantiator` after the user approves the plan.

## Input Shape

Accept unrefined inputs such as:

- pasted conversations
- stability or readiness assessments
- cross-repo migration strategy
- partial handoffs
- product or architecture goals
- "what is left before X can consume Y" analysis
- rough work orders that are not yet staged for an orchestrator

Treat the input as raw material, not as already-executable instruction.

## Intelligence And Reasoning

Use the highest available reasoning level for every plan-building run. This step turns ambiguous strategic context into the orchestration structure that other agents will trust, so prefer quality over token or latency savings.

When this skill is run in a created or prompted thread, request the highest supported reasoning level as launch metadata, normally `thinking: xhigh` on current thread tools. Do not set model unless the human explicitly requested one.

Reason explicitly and thoroughly about:

- dependencies between repos, domains, specs, dry-runs, maps, and migrations
- what must be decided before implementation can start
- what problems appear independent, coupled, blocked, or uncertain
- what needs human approval
- what should be kept local versus reusable
- what belongs in root orchestration, record keeping, later live planning, worker prompts, or future product backlog
- where raw strategic context should be compressed into durable plan facts

Do not merely restate the raw input. Convert it.

Do not pre-plan executable work slices, branch names, worker prompts, or delegation order. The live `orchestration-next-work-planner` is responsible for evaluating the current state and deciding what is workable and completable at that time.

## Self-Review

Before presenting the plan, recursively critique and refine it until satisfied that it provides high-perspective clarity without prematurely narrowing execution.

Check:

- Have the real problems to solve been identified?
- Are the relationships and dependencies between problems clear?
- Are assumptions separated from decisions?
- Are solution paths described as possibilities rather than premature assignments?
- Is enough context propagated for later planners to make good choices?
- Is the plan too narrow, too branch-specific, or too implementation-shaped?
- Are human-decision gates explicit?
- Is the record-maintainer seed useful for future refresh?

Only show the final refined plan and a concise note about major self-review adjustments. Do not expose a long internal critique transcript.

## Collaboration

Ask the user only for decisions that block a safe plan. Prefer making labeled assumptions for details that can be adjusted later.

Use this pattern:

1. Extract likely objective, current state, decisions already made, missing decisions, and implied work order.
2. Identify conflicts, unstated assumptions, and unsafe ambiguities.
3. Ask a short clarification question only if the plan cannot be safely drafted.
4. Produce a draft orchestration-ready plan.
5. Ask the user to approve, revise, or defer instantiation.

Treat vague affirmative approval as approval. If the user says something like "alright, let's do this", "sounds good", "go ahead", or "yes" after seeing the plan, proceed to `orchestration-instantiator` unless a specific unresolved blocker makes instantiation unsafe.

If the user's response is genuinely ambiguous about whether to approve, revise, defer, or only discuss the plan, ask a concise clarification before instantiating.

When proceeding, state the approval assumption briefly and hand off the approved plan plus record-maintainer seed material to `orchestration-instantiator`.

## Plan Contents

Produce an orchestration-ready high-level plan with:

- plan title and slug
- objective
- current state
- accepted decisions
- non-goals and guardrails
- repo/project scope
- participating repo roles: change targets, read-only context repos, and out-of-scope repos
- target stable state
- required durable files
- orchestration home requirements
- participating repo locator policy
- root thread topology
- problem architecture
- problem relationships and dependencies
- possible phase boundaries
- clues about likely sequencing or parallelism, clearly marked as clues rather than work orders
- validation gates
- human-intervention gates
- record-maintenance expectations
- record-maintainer seed material
- initial root-orchestrator prompt inputs
- initial root-record prompt inputs
- risks and open questions

## Structured Plan Object

Include a product-oriented `orchestrationPlanDraft` JSON object with the final plan. The object is a semantic planning artifact for user approval, not a file write. Use stable ids and nested nodes so Codex Orchestrator can later render where the project sits in the proposed problem structure.

Use this shape:

```json
{
  "schemaVersion": 1,
  "slug": "plan-slug",
  "title": "Human-readable title",
  "objective": "Target outcome",
  "scope": {
    "changeTargets": [],
    "readOnlyContext": [],
    "outOfScope": []
  },
  "planRoot": {
    "id": "plan-root",
    "title": "Overall problem",
    "kind": "objective",
    "summary": "What this node solves",
    "status": "proposed",
    "repoRouting": [],
    "dependencies": [],
    "decisionGates": [],
    "validationConcerns": [],
    "children": []
  },
  "assumptions": [],
  "humanDecisionGates": [],
  "productBlockers": [],
  "recordSeed": {}
}
```

Nested plan nodes should describe problems, stages, and possible sub-stages. They may include clues about likely implementation shape, but they are not work slices. The live planner may modify nodes, add sub-nodes, mark nodes blocked or complete, and attach work-slice lifecycle records as execution reveals reality.

Use `productBlockers` for user-addressable decisions or missing inputs that should appear in the Codex Orchestrator UI. Link each blocker to relevant plan node ids and include the question the product should present to the user. Do not turn ordinary dependencies, validation concerns, or planner-owned choices into blockers.

## Record-Maintainer Seed

Include a section specifically designed for `orchestration-instantiator` to turn into record-maintainer material.

Define:

- high-level map seed: target, current location, done, missing, active blockers
- phase record seed: phase names, goals, dependencies, current status
- decision log seed: accepted choices that future agents must preserve
- problem index seed: problems, relationships, uncertainty, likely gates, and validation concerns
- refresh cues: what future intake and context-compression-refresh should read first
- pruning guidance: raw input details that should be summarized, linked, or dropped
- human-intervention gates: decisions the record root must keep visible
- product blocker seed: user-addressable decisions with plan-node links, resolution questions, and next-planner context

Do not make the record seed an archive of the raw conversation. It should be the durable structure needed for future refresh and record maintenance.

## Orchestration Home

Plan for orchestration-owned data to live outside the workspaces being orchestrated. The current default home is:

```text
~/.codex/orchestrations/<plan-slug>/
```

In future Codex Orchestrator product flows, this should become a product-owned orchestration data directory.

For every participating repo, decide whether it should receive a local gitignored locator file, normally:

```text
.codex-orchestrator/orchestration-link.json
```

The locator file should point to the orchestration home and should not contain the full orchestration archive. Use it only so agents can rediscover the orchestration home if they start inside a repo or after context compression.

## Problem Area Shape

For each identified problem area, define:

- problem title
- repo routing: where changes would likely land, which repos are read-only context, and which repos are out of scope
- why it matters
- relevant context discovered from the raw input
- related problems
- known dependencies or blockers
- likely source materials to inspect later
- decisions needed before implementation, if any
- possible solution directions, without choosing a work slice
- validation or evidence concerns
- whether it appears independent, coupled, blocked, or uncertain

Do not produce branch names, worker prompts, implementation assignments, or a fixed execution order. Leave that to `orchestration-next-work-planner` and `work-slice-delegation` inside the live orchestration loop.

## Output Length

Prefer thoroughness over brevity. The plan should be long enough to preserve strategic clarity for a future orchestrator, especially for cross-repo or migration work. Avoid filler, but do not compress away dependency reasoning, problem relationships, assumptions, or human-decision gates.

## Output Contract

Return:

- plan readiness: draft, needs user decision, or approved-ready
- orchestration-ready plan
- `orchestrationPlanDraft` JSON object
- record-maintainer seed material
- orchestration home and repo locator requirements
- assumptions
- questions for the user, if any
- recommended instantiation package
- next step: revise plan, approve plan, or use `orchestration-instantiator`
