---
name: sprint-handoff
description: Package one bounded project Sprint and hand it to a fresh Sprint Runner task, including evidence, authority, known ambiguities, and manual or automatic decision policy. Use when the user asks to hand off, begin, transfer, or prepare a Sprint, or when a completed Sprint needs a clean successor context. This skill creates the Sprint Runner task but does not plan or implement the Sprint.
---

# Sprint Handoff

Create a clean authority and evidence boundary for one Sprint, then start its Runner in a fresh task.

## Preserve the distinction

- Treat a **Sprint** as a bounded implementation period operating under assumptions stable enough to act on.
- Treat **problems/concerns** as things the Sprint must resolve.
- Treat **Work Units** as projected executions that may resolve those concerns.
- Do not invent the Work Units in the handoff. Pass hypotheses as hypotheses and leave decomposition authority to the Planner.
- Do not implement project work in the handoff task.

## 1. Establish the Sprint boundary

Identify:

- concise Sprint name;
- intended movement or outcome;
- why this Sprint should begin now;
- stopping or re-evaluation condition;
- authority granted to the Planner;
- decisions reserved for the user or parent Epic thread;
- whether decision authority is manual or delegated during automatic Epic execution;
- explicit non-goals.

If the boundary is genuinely unclear and local evidence cannot resolve it, stop your work, reject the epoch and respond to the user asking for clarification.

## 2. Inspect current reality

Read the sources needed to ground planning:

- relevant conversation decisions and attached briefs;
- maintained plans, handoffs, evidence, and architecture records;
- repository branch, status, recent history, structure, and relevant tests;
- directly relevant prior-Sprint outcomes;
- known blockers, surprises, rejected approaches, and open product decisions.

Preserve source paths, task IDs, commit IDs, and validation results that the Planner may need to verify.

Distill durable decisions and clue about where evidence can be found. Let the planner determine it's own source ingestion.

## 3. Build the handoff packet

Use this structure:

### Sprint frame

- Name
- Objective
- Why now
- Completion/re-evaluation condition

### Authority and boundaries

- Planner authority
- User/parent-reserved decisions
- Manual or automatic decision policy and the durable decision-record location
- In scope
- Non-goals
- Safety and operational constraints

### Stable context

- Accepted product and architectural decisions
- Definitions and terminology that must remain distinct
- Assumptions considered stable for this Sprint

### Current reality

- Repository/project state
- Relevant completed work
- Existing behavior and evidence
- Known debt, blockers, surprises, and contradictions

### Problem signals

List observed concerns without prematurely turning them into Work Units. Include uncertainty and competing interpretations.

### Known decision points

List consequential ambiguities already visible, their latest safe decision point, and any decisions
that remain human-reserved even during automatic execution. Leave discovery of additional ambiguity
to the Planner.

### Existing projections

Include prior maps or candidate sequences only as inherited projections. State which portions are accepted, provisional, or obsolete.

### Source index

List paths, tasks, commits, and records to inspect.

## 4. Start the Planner

Use the Codex project/thread creation tool when available.

- Create a fresh project task in the relevant repository checkout.
- Use `gpt-5.6-sol` with `medium` reasoning by default.
- Instruct the new task to use `$sprint-runner`.
- Include the complete handoff packet in the initial prompt.
- Tell the Planner to inspect cited evidence rather than trusting the packet blindly.
- Keep the Sprint Runner in planning authority; do not ask it to implement the Sprint during its first turn.

If task creation is unavailable, produce the exact launch prompt and clearly state that no Planner was started.

## 5. Report the handoff

Return:

- Sprint name and objective;
- Planner task ID/link;
- model and reasoning;
- important assumptions or omissions;
- any blocking condition.

Emit the application’s created-thread directive only after task creation succeeds.
