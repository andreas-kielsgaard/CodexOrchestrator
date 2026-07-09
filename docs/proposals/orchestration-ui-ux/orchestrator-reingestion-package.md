# Orchestrator Re-Ingestion Package

## Purpose

This package is for the root coordination thread after context compaction, thread drift, or a long-running worker cycle. Re-ingest it before launching, reviewing, or accepting additional work on the Orchestration UI/UX proposal.

The root thread owns coordination and final decisions. Implementation should happen in focused worker threads unless a task is tiny, administrative, or explicitly safer in the root.

## Root Objective

Coordinate implementation of the Orchestration UI/UX change proposal for the Codex Orchestrator repo.

Proposal files:

- `docs/proposals/orchestration-ui-ux/overview.md`
- `docs/proposals/orchestration-ui-ux/01-truthful-state-model.md`
- `docs/proposals/orchestration-ui-ux/02-reusable-ui-storybook-foundation.md`
- `docs/proposals/orchestration-ui-ux/03-orchestration-client-contract.md`
- `docs/proposals/orchestration-ui-ux/04-add-orchestration-refactor.md`
- `docs/proposals/orchestration-ui-ux/05-runtime-integration.md`
- `docs/proposals/orchestration-ui-ux/06-attention-flow-verification.md`

## Non-Negotiable Product Rule

Do not let workers fabricate orchestration runtime details.

The UI must not invent:

- agent progress
- generated files
- thread IDs
- plan-builder output
- instantiator output
- root startup completion
- live run status
- stage completion

Unfinished behavior should be represented honestly as draft, ready, pending, unsupported, integration-pending, waiting for event, no output yet, or mock/demo.

Mock/demo fixtures must be visibly and structurally distinguishable from real runtime data.

## Recursive Coordination Protocol

1. Maintain root ownership of sequencing, review, and acceptance.
2. Launch focused worker threads for implementation slices.
3. Include this root thread ID in every worker prompt so workers can report back: `019f3e20-5772-7962-b3da-47590da52f9a`.
4. Worker completion reports should include summary, files changed, tests or verification run, blockers, branch or commit if any, and review notes.
5. Review each completed worker before proceeding.
6. If a worker result is not acceptable, re-prompt the worker or create a correction worker with the same report-back instruction.
7. Only unblock downstream slices after the relevant upstream contracts are accepted.
8. Keep docs and coordination state honest. Do not mark planned work complete before review acceptance.

## Parallelism Guidance

Parallelize evidence-gathering and preparation. Serialize decisions that define shared truth.

Safe root-side parallel tasks:

- inspect likely affected files while a worker runs
- prepare next worker prompts as drafts
- prepare review checklists
- ask a helper thread to independently review a completed slice
- update coordination notes after acceptance
- prepare future verification scenarios without claiming they have passed

Decisions that should remain serial in the root:

- accepting or rejecting a worker result
- approving shared state names and client contracts
- approving reusable UI APIs that downstream work will depend on
- deciding that Slice 4 is unblocked
- merging or signing off on changes
- resolving cross-worker conflicts

Suggested wave structure:

- Wave 1: Slice 1 plus optional Slice 2A.
- Wave 2: Slice 2B plus Slice 3 after Slice 1 is accepted.
- Wave 3: Slice 4 plus optional Slice 5A after Slice 2/3 prerequisites are accepted.
- Wave 4: Slice 5B runtime integration.
- Wave 5: Slice 6 attention-flow verification.

Definitions:

- Slice 2A: Storybook configuration, generic UI primitives, component folder structure.
- Slice 2B: orchestration-specific stories and components that depend on the truthful state model.
- Slice 5A: backend persistence/client adapter preparation that does not claim runtime progress.
- Slice 5B: real runtime integration.

## Reasoning Level Guidance

Use higher reasoning for decisions that establish shared contracts or cross-boundary behavior. Use medium or lower reasoning for mechanical implementation, isolated UI extraction, and documentation cleanup.

Recommended levels:

- Root orchestration sequencing: high.
- Root acceptance review for shared contracts: high or xhigh.
- Slice 1 Truthful State Model: high, because it defines product truth and prevents misleading UI behavior.
- Slice 2A Storybook setup and generic primitives: medium, unless dependency/config conflicts appear.
- Slice 2B orchestration components/stories: medium to high, depending on how tightly they encode truthful state semantics.
- Slice 3 OrchestrationClient contract: high, because it shapes frontend/backend boundaries and downstream work.
- Slice 4 Add Orchestration refactor: high, because it combines UX, state truthfulness, app architecture, and regression risk.
- Slice 5A persistence/client adapter prep: high if it touches contracts or storage, medium if it is a narrow adapter/test slice.
- Slice 5B real runtime integration: xhigh for design/review and high for implementation workers, because it crosses UI, Tauri, persistence, runtime events, and user trust.
- Slice 6 attention-flow verification: medium for checklist execution, high for final UX interpretation and acceptance.
- Independent review helper threads: high for code review of shared contracts or runtime integration; medium for UI/story coverage checks.
- Documentation-only updates: low or medium, unless they define process or acceptance criteria.

Do not use high reasoning as a substitute for scope control. Prefer small slices with crisp acceptance criteria.

## Current Critical Path

The truthful state model is the guardrail. Do not let downstream workers invent state names, provenance semantics, or runtime claims before Slice 1 is accepted.

The Add Orchestration refactor should wait for:

- accepted truthful state vocabulary
- sufficient reusable UI primitives
- accepted `OrchestrationClient` contract or a clearly scoped interim contract

Runtime integration should not present future capabilities as complete. If backend support is absent, return and display unsupported or integration-pending states.

## Review Checklist For Every Slice

- Did the worker preserve the truthfulness rule?
- Are mock/demo fixtures labeled clearly?
- Does any UI copy imply runtime progress without real backing?
- Are shared contracts introduced in the right layer?
- Are tests or stories focused on the states that matter?
- Did the worker avoid unrelated refactors?
- Are downstream dependencies clearly documented?
- Is the next slice actually unblocked?

## Recovery After Compaction

After compaction, do this before acting:

1. Read this file.
2. Read `overview.md`.
3. Read the current active slice document.
4. Inspect git status and recent worker reports.
5. Reconstruct which slices are accepted, in review, blocked, or not started.
6. Continue from the latest accepted coordination state, not from the beginning.
