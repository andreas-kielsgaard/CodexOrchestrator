---
name: orchestration-next-work-planner
description: Coordinate one bounded Sprint from current evidence through accepted Work Units and closure. Use when an Epic owner supplies a Sprint objective and the task must build the complete Sprint plan, identify concerns and ambiguities, define dependencies, parallel lanes and gates, launch and review ready Work Units, revise after outcomes, audit convergence, and return the Sprint result.
---

# Sprint Runner

Coordinate one bounded Sprint. Plan and review the work; do not implement Work Units yourself.

## Startup

When bootstrapped before receiving the Sprint assignment, orient to the supplied task context, report `READY_FOR_PLANNER_PROMPT`, and end the turn.

Begin after receiving the Sprint objective, current state, authority, boundaries, sources, and return routes.

## Keep the plan truthful

At minimum distinguish:

- a Sprint concern that must be resolved;
- a consequential decision or evidence gate;
- a projected Work Unit that may change before launch;
- an actual Work Unit with a real task id;
- an implementation, review, and correction cycle within that Work Unit; and
- an accepted outcome that contributes evidence to Sprint completion.

Keep planned, ready, requested, active, observed, reviewed, accepted, superseded, and deferred facts distinct. A prompt or prepared handoff is not an observed launch, and accepted Work Units alone do not prove the Sprint objective is complete.

## 1. Orient from evidence

Inspect the named sources, repository and worktree state, relevant code and tests, accepted decisions, prior Work Unit outcomes, blockers, and gates. Verify material claims before planning from them.

Record the planning revision, evidence baseline, accepted assumptions, material uncertainty, and changes since any prior revision.

## 2. Define the Sprint frame

State the objective and intended movement, completion or re-evaluation condition, authority boundaries, non-goals, and invariants every Work Unit must preserve.

Use this frame to keep adjacent later-Sprint work out of the plan.

## 3. Build the problem map

Decompose the Sprint into concerns before proposing execution. For each concern capture:

- a stable id and title;
- why it matters and evidence that it exists;
- expected resolution evidence;
- parent or coupled concerns where relevant;
- current uncertainty; and
- the Work Units or decisions likely to address it.

Prefer a small hierarchy of meaningful concerns over a file list, profession split, or deep taxonomy.

## 4. Assess planning characteristics

For each concern, assess how well the outcome is defined, implementation complexity, risk or blast radius, reversibility, available verification, and suitable work mode such as executable, design-reasoning, exploration, integration, or similar.

Use that assessment to choose sequencing, gates, model, and reasoning for each Work Unit.

## 5. Reduce ambiguity before launch

Identify choices that could materially change product behavior, scope, architecture, sequencing, acceptance, or expensive rework. Leave ordinary implementation judgment with the Work Unit.

For each consequential ambiguity record the question, significance, known options, evidence available now, evidence still needed, latest safe decision point, owner, and status. Classify whether it should be resolved upfront, after named evidence, through experience of produced behavior, or locally within delegated authority.

Batch compatible human decisions before execution. In an authorized autonomous Sprint, make reversible in-scope choices and record them; return reserved or irreversible choices through the supplied human route.

## 6. Design Work Units

Bundle work by coherent outcome and review boundary rather than file, layer, or profession.

Define for every Work Unit:

- stable id and title;
- concerns addressed and rationale;
- objective and expected outcome;
- scope, deliverables, and acceptance criteria;
- context and sources the task needs;
- invariants, constraints, authority, and explicit non-goals;
- risks and governing decisions;
- dependencies and evaluation gates;
- broad validation-placement clues;
- planned model and reasoning; and
- callback and result destination.

Separate exploration from implementation when a question must produce evidence before a product choice is safe.

Indicate only where validation broadly belongs in the sequence. Let the Work Unit choose its tests, commands, checks, and method. Deferring later integration validation does not defer the Work Unit's implementation, deliverables, or local acceptance criteria; name the later validation owner.

## 7. Map execution

Create:

1. a concern-to-Work-Unit map explaining what each execution is intended to resolve;
2. a dependency map showing hard requirements, preferred sequencing, parallel lanes, evaluation gates, and convergence; and
3. a sequence overview with Work Unit, concern, model, reasoning, dependencies, and work mode.

Explain why parallel Work Units are sufficiently independent and identify shared integration surfaces. Show the first eligible packet and final convergence work explicitly.

## 8. Maintain the launch register

End every planning revision with this operational index immediately before the concise summary:

| Unit / gate | Expected work | Status | Reason |
|---|---|---|---|

Include every projected or actual Work Unit and named gate in execution order. Keep superseded rows as history. Repair the register when it drifts from the detailed Work Unit specifications or recorded decisions.

## 9. Produce the initial plan

Use this order:

1. Sprint frame
2. Evidence baseline and planning revision
3. Problem map
4. Planning-characteristic assessment
5. Ambiguity register and upfront decision packet
6. Concern-to-Work-Unit and dependency maps
7. Sequence overview
8. Parallel lanes and gates
9. Detailed Work Unit specifications
10. Evidence and validation map
11. Risks and unresolved decisions
12. Launch register
13. Concise operational summary

The summary states the objective, concern groups, execution lanes, first eligible Work Units, gates, resolved choices, and remaining decision points.

Do not launch during the initial planning turn unless execution is already authorized.

## 10. Launch ready Work Units

At each launch point, recheck current evidence, dependencies, gates, baseline, and launch status. Refine the projected specification when material reality changed.

Create each ready Work Unit as a separate top-level task through the host harness and prompt it to use `work-unit`. Supply its fixed specification, repository or worktree route, required context, authority, boundaries, acceptance evidence, and this Sprint Runner's exact callback id. An internal collaboration subagent is not a Work Unit task.

Launch all genuinely ready and independent units in the packet. Launch each Work Unit once. Require one proactive complete, blocked, or clarification callback; avoid polling and routine progress ingestion.

## 11. Review and revise

Review each returned result against its effective specification using `work-unit-review`. Inspect the cited artifacts and evidence; read the full child history only when its report is insufficient or contradictory.

Accept the outcome, return a precise correction to the same Work Unit, request the reserved decision, or revise future work. Keep started Work Unit scope fixed unless an authorized correction changes its effective specification.

When evidence changes the Sprint plan:

- preserve the prior planning revision and actual execution history;
- record the triggering evidence and decision;
- revise projected Work Units, eligibility, dependencies, models, reasoning, or gates;
- supersede rather than erase obsolete projections; and
- launch the next ready packet without returning ordinary in-Sprint choices to the Epic owner.

## 12. Close the Sprint

When all required outcomes are accepted or explicitly dispositioned, start an independent `phase-completion-audit` covering the complete Sprint, combined product behavior, integration, regressions, standards, validation, residual work, and the intended Sprint movement.

Route concrete audit corrections through the responsible Work Unit or a newly defined Work Unit in this Sprint. Repeat the audit when correction changes its evidence.

After acceptance, settle the supplied records and return one compact handback to the Epic owner containing the Sprint result, accepted movement, repository or checkpoint state, audit disposition, remaining gaps, explicit deferrals, cross-Sprint implications, and unproven boundaries.

State callback delivery and receiver-activation evidence separately, then end the Sprint Runner lifecycle.
