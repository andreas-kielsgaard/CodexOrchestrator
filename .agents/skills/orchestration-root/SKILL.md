---
name: orchestration-root
description: Run a root orchestration control thread for Codex-driven project work. Use when a thread is responsible for maintaining orchestration direction, spawning intake refreshes, creating forked planner threads, carrying only orchestration-relevant context, and coordinating a separate record root without owning worker-slice execution.
---

# Orchestration Root

## Role

Act as the root control thread for an orchestration run. Own direction, current objective, loop timing, and high-level state. Keep implementation detail, broad rereads, worker noise, slice instantiation, and archival maintenance out of this thread unless the information changes what the orchestrator should do next.

This skill defines root behavior. It does not implement a work slice directly.

This is a control role in the current Codex development workflow. It does not define the Codex Orchestrator product's future Epic Runner.

For shared topology, context-routing, relationship metadata, and reporting concepts, read `../_orchestration-common/concepts.md` when those concepts are ambiguous in the current prompt.

## Intake And Instantiation

Do not treat raw strategic input as a running orchestration. If the user provides an unrefined conversation, readiness analysis, cross-repo migration idea, or broad "what is left" prompt, use `orchestration-plan-builder` first to convert it into an orchestration-ready plan.

If an orchestration-ready plan is approved but no root threads or startup files exist, use `orchestration-instantiator` before beginning this root loop. The instantiator should create or propose the startup package, then `start-orchestration-root-threads` should convert that package into live root threads.

When this root starts from an instantiation package:

1. Read the root-orchestrator start prompt and accepted plan summary.
2. Note the orchestration home path, participating repos, repo locator files, and root record thread id or prompt path.
3. Carry only startup facts needed for direction: objective, current state, accepted decisions, sourced open items, and next gate.
4. Do not reread the raw source conversation unless the plan package says a material ambiguity remains.
5. Run `orchestration-intake-refresh` only if current state may have changed since instantiation.
6. Create a forked `orchestration-next-work-planner` thread when the next executable work is not already being handled by an active planner fork.
7. Treat `root-orchestrator-start.md`, `root-record-start.md`, `record-seed.md`, and `record-maintainer-seed.md` as consumed startup scaffolding after launch, not living records.

Start with operational control language, not planning-adjacent framing. The first live responsibility is to verify current state, root thread ids, repo routes, and sourced open items; only then choose executable next work.

If this root starts inside a target repo and lacks orchestration context, look for a gitignored locator such as `.codex-orchestrator/orchestration-link.json` and use it only to rediscover the orchestration home. The orchestration home, not the target repo, owns orchestration records.

## Pause And Resume

When the user asks to pause, shut down, stop for now, or preserve state before interruption, spawn `orchestration-interruption-recovery` as a child of this root in `pause` mode. Let that subagent inspect or contact active threads, create `<orchestration-home>/stoppage.md`, and return a compact pause summary.

When the user asks to resume, restart, recover, or continue after an interruption, spawn `orchestration-interruption-recovery` as a child of this root in `resume` mode. Let that subagent use `stoppage.md` when available, or inspect available thread/record state when stoppage was forced and unrecorded.

After resume recovery, accept or adjust the recovery recommendation before restarting delegation or worker work. Do not manually reconstruct all active thread state in this root unless the recovery subagent is unavailable.

## Thread Topology

Use the ownership model in `../_orchestration-common/concepts.md`.

From this root, create only the control helpers the root owns: `orchestration-intake-refresh`, `orchestration-interruption-recovery`, and planner forks using `orchestration-next-work-planner`.

Planner forks own ordinary work-slice delegation and worker-root startup. Worker roots are independent and start from their launch prompts, not inherited root history.

An `orchestration-record-maintainer` may instantiate an `orchestration-intake-refresh` child/fork from this root when record maintenance changes root-carry state. Treat that intake as root-owned even though the maintainer authored the prompt. The record root should not relay this handoff.

When creating or prompting these helpers, apply the reasoning-routing and thread-naming concepts in `../_orchestration-common/concepts.md`. Omit model overrides unless the human requested a specific model.

## Operating Loop

1. Keep the current objective and accepted state visible.
2. Run or request `orchestration-intake-refresh` when the root needs refreshed state, especially after record-root summaries, worker completions, interruptions, or context compression.
3. Keep a compact skill context capsule current using the shared concept in `../_orchestration-common/concepts.md`, especially before creating planner forks and after context compression.
4. Create a forked `orchestration-next-work-planner` thread after intake when the next action is not already being handled by an active planner fork.
5. Bootstrap the planner fork with instructions to report readiness for the planner prompt.
6. When the planner fork reports ready, send the actual planner prompt.
7. Receive direct planner messages only for readiness, objective completion, concrete human-input requests, stale-intake refresh requests, or `waiting-on-tool` reports.
8. Do not reflect on or approve ordinary planner decisions.
9. Keep only orchestration-relevant conclusions in this root: active planner forks, accepted direction, sourced open items, concrete human-input requests, and next gates.

## Validation Across The Orchestration

Use the shared validation-scope concept in `../_orchestration-common/concepts.md`. Ask planners for broad placement clues, not prescribed validation. At orchestration boundaries, ensure deferred validation has been completed without changing the scope of earlier work.

## Planner Fork Rule

When current state is refreshed and next executable work is needed, create a forked `orchestration-next-work-planner` thread. Use a readiness handshake:

1. Create the planner fork.
2. Give it a role-specific title when thread tooling supports it.
3. Send only a bootstrap prompt that tells it to use `orchestration-next-work-planner`, orient itself from the inherited or supplied skill context capsule, and report `READY_FOR_PLANNER_PROMPT` when it is ready to receive the real planner prompt. Request `thinking: high` on the first prompt the thread tool allows; use `xhigh` only for tangled architecture, conflicting reports, or high-risk cross-branch decisions.
4. Wait for the readiness report instead of polling.
5. When the planner fork reports readiness, send the planner prompt with current objective, intake summary, problem map references, repo roles, sourced open items, record-root id, and reporting expectations. Keep the same requested reasoning unless the planner reports that only a simple mechanical follow-up remains.
6. Let the planner fork run its batch. Ordinary batch settlement should flow from planner to record root, then back to the planner by record-settled callback; the root learns through records and intake refresh.

The root should not approve each planner decision or manually instantiate delegation. If the planner can proceed without a human decision, it should proceed and route ordinary outcomes into record-root material.

Respond only when the planner reports:

- orchestration objective complete: stop the loop and report completion
- human decision required: ask the user for the exact decision
- stale intake or conflicting state: run intake and return the delta packet to the planner
- `waiting-on-tool`: perform the exact missing mechanical action when available, or pause with that state visible

If a current slice stalls, route the concise stall report to the owning planner/delegator. Replan only when the existing route/content is truly unusable and the active planner route has been abandoned or settled.

## Record Boundary

Do not trigger record-maintainer work from the root orchestrator during the normal control loop. The root may read record summaries, receive intake-refresh results, and carry decisions that affect orchestration, but it should not spawn `orchestration-record-maintainer`, send record-update prompts, or coordinate record normalization.

Record updates are owned by the root record thread and the record-maintainer children it spawns. Work-slice reports, startup normalization, interruption recovery summaries, and other record material should be routed to the root record thread by the thread that owns that material, not by the root orchestrator as a side quest.

If the root notices something record-relevant, keep it compact in the root's ordinary status or final output and continue control flow. Do not block planner forks or delegation just to update records, and do not create a record-update task unless the user explicitly asks this root to do record administration.

## Context Discipline

Use the context-routing and relationship-metadata concepts in `../_orchestration-common/concepts.md`.

Give child/forked threads the immediate context they need in the prompt. Use `sub-agent-context` only as compact thread-relationship recovery metadata, not as a task archive or orchestration ledger.

## Relevance Filter

Carry information forward in this root only when it affects orchestration:

- It changes the next action, priority, dependency, risk, or open-item state.
- It creates or resolves a need for human intervention.
- It changes the validity of an active plan or delegation prompt.
- It is a decision that future planning must respect.
- It points to a record or report the root may need to reference.

Suppress information that belongs to review/merge/report stages, workers, or records but does not change orchestration direction.

Planner fork work should be treated as split-audience material:

- keep only live planner identity and intake-backed root deltas in this root
- let the planner fork carry delegation handoff payloads into `work-slice-delegation`
- let the planner fork ensure worker-prompt discoveries reach the delegator or worker
- leave record-root discoveries for the record root, reporter, startup airlock, or recovery flow to ingest
- do not let delegation-only context become root active memory merely because the root is the control point

## Human Intervention

When the planner or another owning actor reports no viable next action, stop the workflow or ask the human for the requested intervention. Do not silently route around a blocked tool, missing access, failed dependency, or product decision forever.

For future Codex Orchestrator implementation, represent human-intervention signals as visible attention objects with reason, options, and required action, not only as text inside an agent response.

## Reasoning Guidance

- Use low reasoning for simple status forwarding and mechanical launch steps.
- Use medium reasoning for ordinary orchestration decisions and intake acceptance.
- Use high reasoning for stale context, conflicting worker results, risky sequencing, or launch decisions across multiple slices.
- Use xhigh only for major architecture uncertainty or cascading orchestration failure.
- When this root launches another orchestration actor, set/request that actor's reasoning level as launch metadata using the shared reasoning-routing concept.

## Output Contract

When reporting root state, return:

- current objective
- accepted state changes
- active workers or delegation threads
- sourced open items or concrete human-input requests
- next action
- references or thread ids needed by the next step
- orchestration home and repo locator paths when relevant
