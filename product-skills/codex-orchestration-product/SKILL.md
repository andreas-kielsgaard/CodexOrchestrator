---
name: codex-orchestration-product
description: Design or refactor Codex Orchestrator product skills around product-owned orchestration state, detached Codex conversations, prompt artifacts, event-driven routing, and UI-visible orchestration history. Use when updating orchestration skill concepts, product workflow design, or implementation plans so the app/server owns topology and lifecycle instead of long-running semantic root conversations.
---

# Codex Orchestration Product

## Role

Shape orchestration skills and product behavior for Codex Orchestrator as a first-class control plane.

The product owns orchestration topology, lifecycle state, prompt routing, history, records, and UI navigation. Codex conversations are execution resources: they receive bounded prompt packets, perform one role-specific turn or stage, and return outputs/events for the product to route.

Use this skill to revise orchestration-related skills, prompts, UI concepts, persistence plans, or runtime-controller behavior toward that model.

## Product-Centered Model

Prefer this architecture:

1. The product stores the orchestration graph.
2. The product creates prompt packets as artifacts.
3. The product starts, resumes, forks, interrupts, and monitors Codex conversations through available local Codex control surfaces.
4. Conversations return raw events, outputs, diffs, review decisions, merge results, and report material.
5. The product advances lifecycle state deterministically from those outputs.
6. The UI renders orchestration overview, planner history, work-slice timelines, recording activity, and prompt/output details from product state.

Do not design future skills around long-lived root conversations remembering their own graph. A conversation may reason locally, but the product remains the source of truth for relationships and next routing.

## Skill Rewrite Direction

When adapting existing orchestration skills, keep the current skill usable but move its concept center:

- `orchestration-root`: becomes a product controller policy, not a memory-heavy conversation role.
- `orchestration-record-root`: becomes record/projection maintenance owned by the product, with optional Codex turns for summarization or pruning.
- `orchestration-next-work-planner`: becomes a planner-turn generator that returns structured planning output and prompt packets for selected work.
- `work-slice-delegation`: becomes slice lifecycle coordination in product state, with Codex used for prompt construction, review, merge reasoning, or worker launch material.
- `orchestration-worker`: remains a bounded independent execution conversation supplied by a complete prompt packet.
- `review-before-merge`, `merge-accepted-work`, and `merge-reconciliation`: become stage prompts tied to a work-slice lifecycle record.
- `work-slice-reporter`: becomes an adapter from stage outputs into a compact completion report and record update packet.
- `orchestration-record-maintainer`: becomes an adapter from completion/update packets into maintained projections.
- `orchestration-intake-refresh` and `context-compression-refresh`: shrink in importance; product state and event logs supply orientation, while Codex refresh turns can summarize deltas for humans or planner prompts.
- `orchestration-interruption-recovery`: becomes product-run reconciliation against persisted lifecycle state and observed Codex thread/turn status.

Keep active production skills stable until the user explicitly asks to replace them. Use this repo-local skill as the staging philosophy for future edits.

## Prompt Packet Contract

Design role prompts as product-generated packets with explicit fields:

- orchestration id and title
- role and lifecycle stage
- target conversation/thread id when continuing an existing conversation
- parent graph node id or source decision id
- destination for returned output
- repo/worktree route, if any
- relevant context refs and inline context
- acceptance criteria or decision question
- expected output schema or headings
- allowed actions and verification expectations

Put immediate task context directly in the prompt packet. Use files or artifacts for durable evidence, records, reports, and large references. Do not rely on a conversation remembering broad orchestration history.

## Product State Vocabulary

Model these as first-class product records:

- orchestration
- orchestration plan/problem map, stored as nested product-readable JSON
- planner turn
- planner decision
- work slice
- lifecycle stage
- prompt artifact
- Codex conversation
- Codex turn
- raw event stream
- stage output
- repository integration result
- completion report
- record/projection update
- human decision request
- product blocker and user conclusion
- interruption/recovery event

Prefer product state transitions over conversational callbacks.

## UI Expectations

The UI should make orchestration relationships legible:

- orchestration shell: objective, anchor repos, current position, and status counters
- plan map: nested proposed problems/stages with state, blockers, active work, and completion
- live state: currently running planner, worker, review, merge, report, and record windows
- blockers: decision records linked to plan nodes or work slices, with a detail view for user conclusions
- history: planners first, work slices under planners, lifecycle stages under slices
- detail inspector: prompt packet, full output, raw event links, diffs, validation, decisions
- recording: visually near the slice it records, but distinct from the execution timeline
- active work: show planner/worker/review/merge/report/record state without requiring the user to inspect threads

The user should be able to answer: where are we, what is running, why was this slice created, what happened, what is next, and what evidence backs that state.

## Implementation Notes

When implementing this product philosophy in the repo:

1. Add domain records before UI plumbing when persistence is in scope.
2. Capture raw Codex app-server or `codex exec --json` events before deriving projections.
3. Keep prompt artifacts and output artifacts linked to lifecycle stages.
4. Let the controller route the next prompt after a stage completes.
5. Treat local Codex app-server as the richest near-term control surface; keep direct OpenAI API features as future runtime concepts unless API billing is explicitly accepted.
6. Preserve the existing open-task runtime path while adding orchestration-specific records and projections.

## Reference

Read `references/product-orchestration-model.md` when making nontrivial edits to orchestration skills or implementation plans from this philosophy.
