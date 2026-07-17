---
name: orchestration-next-work-planner
description: Run a forked next-work planner for an active orchestration run. Use when a root orchestrator creates a planner fork after intake refresh; report readiness for the planner prompt, decide executable work, use work-slice delegation directly for a single slice or fork planner-owned delegation paths for parallel slices, track launched slices, and route ordinary batch settlement to record-root material with a callback route rather than direct root reporting.
---

# Orchestration Next Work Planner

## Role

Run as a forked planner thread for the root orchestrator during an active orchestration. Decide what executable work should happen next, own delegation for accepted work without returning to the root for approval, track the planned batch, and route settled batch state into record-root material with a callback route.

This skill does not build the high-level orchestration plan. Use `orchestration-plan-builder` for raw strategic input and problem architecture before orchestration starts.

This is an ad-hoc planning role in the current Codex development workflow, not the product's future Planner or Work Unit model.

Do not wait for the root to accept ordinary next-work decisions. The planner owns its batch.

Directly ask the human when a concrete decision or resource is required. If the orchestration objective appears complete, report completion and stop the batch.

For one selected immediate slice, continue this planner thread with `work-slice-delegation` and stage one independent worker root from there. For multiple independent immediate slices, fork this planner thread once per parallel slice and prompt each fork to use `work-slice-delegation` for exactly one slice. Do not fork for a single slice.

For shared ownership, owner-liveness, context-routing, work-route, and reporting concepts, read `../_orchestration-common/concepts.md` when the prompt does not already make those concepts clear.

Apply the shared reasoning-routing and naming concepts when starting delegation continuations, planner forks, and worker roots. Omit model overrides unless the human explicitly requested a model.

Delegation startup uses this planner thread for a single slice and planner-thread forks for parallel slices. When the needed thread action is unavailable, produce the exact `work-slice-delegation` continuation or fork prompt, mark the slice `waiting-on-tool`, and wait for a usable thread path before launch.

## Startup Readiness

When first bootstrapped by the root, do not begin planning from partial context. Orient to the forked thread, inherited or supplied skill context capsule, and shared concepts, then report:

```text
READY_FOR_PLANNER_PROMPT
```

Then wait for the root's actual planner prompt. The root should not manually probe for readiness; readiness must be reported proactively.

## Inputs

Expect:

- current objective and project scope
- problem map and participating repo roles
- latest intake-refresh summary
- active workers or branches
- relevant roadmap or phase state
- constraints, blockers, and human preferences

If available, create or update a compact thread-relationship `sub-agent-context` record keyed by this planner thread id for compaction recovery. Do not store the planning payload, reasoning, or project context there.

## Next-Work Planning Rules

Be decisive. Return a recommended course of action, not a menu of equally weighted options.

Identify:

- the next useful slice
- the repo route for each next slice: change-target repo, starting cwd/worktree, read-only context repos, and repos that must not be edited
- independent slices that can be staged in parallel
- dependent slices that must wait
- required review, merge, or record-maintenance steps
- whether `phase-completion-audit` is appropriate
- whether human intervention is needed now

If a direction is blocked or parked, choose one:

- give a clear alternate path and explain why it preserves the objective
- ask the human for a concrete decision or resource
- recommend stopping the workflow until the blocker is resolved

Do not silently work around a blocked tool, missing access, or unresolved product decision forever.

## Context Propagation

Route discoveries by audience using the shared buckets in `../_orchestration-common/concepts.md`.

Do not send ordinary planner decisions or completion summaries directly to the root. Put slice-specific source files, repo routing, validation, constraints, and worker-facing discoveries in delegation handoff payloads and use those payloads when starting `work-slice-delegation` sub-agents. Route settled planner-batch state, sourced decisions, active slice ownership, stale-plan triggers, and future refresh cues into record-root/record-maintainer material so root can ingest them through intake refresh.

When routing planner-batch settlement to the record root, include this planner thread id or notification route and request a compact record-settled callback. After sending the record update request, mark the batch `waiting-on-record-callback` and wait for the record root or maintainer to poke this planner back. Do not poll record files to decide whether maintenance finished.

## Validation Allocation

Use the shared validation-scope concept in `../_orchestration-common/concepts.md` when selecting and sequencing work.

Pass broad validation-placement clues to each slice and let its worker choose the validation. A validation deferral never moves implementation or acceptance scope. Name the later owner when validation is deferred.

## Delegation And Completion Tracking

After deciding an immediate batch:

1. Keep a concise planning decision capsule in this planner thread and, when useful, in record-root material. Do not ask root to approve ordinary next-work decisions.
2. For a single immediate slice, continue this planner thread with `work-slice-delegation`. For multiple parallel slices, fork this planner thread once per slice, set a role/slice title when tooling supports it, and prompt each fork to use `work-slice-delegation`. Request `thinking: medium` by default or `thinking: high` when prompt scope, repo boundaries, or content dependencies are subtle.
3. Instruct each delegation path to start the independent worker root, preserve the source planner thread id and any planner fork id, and pass a compact skill context capsule for the worker and delegation-stage path.
4. Require each work-slice reporter to notify this planner fork when the slice is complete, blocked, signed off without merge, or abandoned.
5. Track launched slices until all are complete, blocked, or escalated. A delegation path that merely produced a worker prompt or started a worker is still active until review, merge/sign-off, reporting, record-root handoff, and planner notification are settled.
6. When all planned slices are settled, write or send a concise planner-batch settlement update to the root record thread/record maintainer. Include only state that affects future planning, sourced unresolved items, active ownership, and refresh cues, plus the planner callback route.
7. Park the batch as `waiting-on-record-callback`. When the record root or maintainer callbacks with record-settled status and no slice sub-agent remains active, archive this planner fork when thread tooling supports it.

If the required continuation or planner fork is unavailable for any immediate slice, emit the complete `work-slice-delegation` prompt for that slice, keep the slice in `waiting-on-tool`, and include the missing thread action as planner-batch tracking. Resume by launching that exact prompt through the available thread path.

Use proactive owner notification rather than polling. Escalate to interruption recovery only after a user-visible interruption or explicit request.

If a delegator cannot start the worker, message the worker for corrections, notify the planner, or route record-root material because tooling is unavailable in that turn, treat the slice as waiting-on-tool rather than complete. Perform the single missing mechanical action yourself when tooling is available, or record the exact missing action as record-root/recovery material. Do not replan or duplicate the worker.

When a worker or delegator reports a blocker, classify it before replanning:

- `slice-blocking`: the needed work route is dirty/conflicting, content the slice genuinely depends on is unavailable, access is missing, validation failed, or scope is ambiguous.
- `attention-only`: moving ref drift, remote default mismatch, stale branch label, or other state that does not prevent continuing from a clean route with the needed content.

For `attention-only` route/ref drift, keep the existing slice owner and continue from the clean route or needed content. Do not create a duplicate slice or ask the root to re-accept the same work.

If a planned slice already has an active worker thread, treat that worker as the owner of the slice until it completes, is abandoned, or the planner explicitly cancels it. Do not instantiate another worker for the same slice.

## Planner Fork Closure

Planner forks are working coordinators, not durable records. Keep them visible while they own active slices, blockers, downstream review/merge/report handoffs, record-callback waiting, or direct human-input requests. Once every launched slice is complete, blocked with human/record-root handoff, abandoned, or otherwise settled, and the record root or maintainer has sent the record-settled callback for batch settlement, archive the planner fork.

Do not archive before:

- active workers, review/merge/reconciliation/report stages, or record-root handoffs have settled
- the planner has routed the batch outcome into record-root material and received the record-settled callback
- any required human intervention has been clearly recorded or handed off

If self-archiving is unavailable after callback, record that the planner is ready to archive as record-root/recovery material. Do not keep completed planner forks open as living records; the record root and slice reports own durable history.

## Work Route Handoff Policy

Separate work route from content dependency using `../_orchestration-common/concepts.md`. In handoff payloads, name the route, any true content dependency, and whether drift is fatal or attention-only.

## Reasoning Guidance

Use high reasoning by default. Use xhigh only when sequencing depends on tangled architecture, conflicting reports, or high-risk cross-branch decisions. Use medium only for simple next-slice selection after a clean intake.

## Output Contract

Return:

- readiness report, when bootstrapped:
  - `READY_FOR_PLANNER_PROMPT`
- batch decision capsule:
  - decision: proceed, wait, ask human, audit phase, review/merge, or stop
  - rationale: brief decisive reasoning
  - accepted assumptions to route into record-root material
  - next accepted slice names, with parallel vs dependent marking
  - blockers, lifecycle state, or direct human-input request
  - stale-plan triggers
  - continuation instruction: delegation launched, wait for human, refresh intake, audit phase, record-root update, objective complete, or stop
- delegation handoff payloads:
  - one payload per proposed slice
  - source planner thread id, if known
  - planner output reference or final-output location, if available
  - slice title and why it matters now
  - repo route: change-target repo, starting cwd/worktree, branch/worktree route, content dependency if any, read-only context repos, and no-edit repos
  - route/ref drift policy: fatal gate or attention-only
  - worker-prompt context and source materials
  - guardrails and non-goals
  - broad validation-placement clue and deferred-validation owner, if any
  - context intentionally excluded from root
- context routing:
  - root-carry discoveries
  - delegation-handoff discoveries
  - worker-prompt discoveries
  - record-root discoveries
  - suppressed details
- delegation actions taken: direct delegation continuation or planner-fork delegation paths created, worker roots started or startup prompts produced, or exact delegation prompts waiting on tool availability
- planner-batch tracking: active slice ids, completion notification route, requested/applied reasoning for launched actors, record-root settlement trigger, planner callback route/status, and archive-on-completion status
