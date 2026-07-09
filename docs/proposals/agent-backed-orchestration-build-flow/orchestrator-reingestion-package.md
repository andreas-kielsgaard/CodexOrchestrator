# Orchestrator Re-Ingestion Package

## Purpose

This package is for the root coordination thread implementing the Agent-Backed Orchestration Build Flow proposal.

Read this file after context compaction, thread drift, worker completion, or before choosing the next slice. It is designed to keep the root thread aligned with the target user flow without carrying the whole original conversation.

## Root Objective

Coordinate implementation of an agent-backed Add Orchestration build flow for the Codex Orchestrator repo.

The target product behavior is:

- user enters source material in Plan Builder intake
- app starts a real agent conversation through a supported runtime route
- reusable conversation view shows actual runtime state
- user reviews and re-prompts the planner in the same conversation
- user explicitly starts instantiation
- Expected Shape appears only after instantiator evidence
- user explicitly initiates live orchestration
- initiation artifacts and created conversations become visible
- user navigates to the normal orchestration view only after a real target exists

## Proposal Files

Read these files first:

- `docs/proposals/agent-backed-orchestration-build-flow/overview.md`
- `docs/proposals/agent-backed-orchestration-build-flow/01-agent-conversation-contract.md`
- `docs/proposals/agent-backed-orchestration-build-flow/02-plan-builder-intake-ui.md`
- `docs/proposals/agent-backed-orchestration-build-flow/03-plan-builder-runtime-start-and-stream.md`
- `docs/proposals/agent-backed-orchestration-build-flow/04-plan-review-reprompt-instantiation.md`
- `docs/proposals/agent-backed-orchestration-build-flow/05-initiation-artifacts-conversation-windows.md`
- `docs/proposals/agent-backed-orchestration-build-flow/06-end-to-end-usability-verification.md`

Related context:

- Existing proposal package: `docs/proposals/orchestration-ui-ux/`
- Recent audit package: `docs/proposals/orchestration-build-flow-usability-audit-2026-07-08-live-pass/`
- Architecture notes around Codex app-server and conversation views: `docs/architecture.md`
- Current Add Orchestration code: `src/app/App.tsx`
- Orchestration client contract: `src/application/orchestrationClient.ts`
- Local orchestration client: `src/infrastructure/localOrchestrationClient.ts`
- Tauri orchestration client: `src/infrastructure/tauriOrchestrationClient.ts`
- Tauri backend commands: `src-tauri/src/lib.rs`

## Non-Negotiable Product Rule

Do not make up orchestration details ahead of time.

Workers must not claim or imply:

- an agent is thinking without runtime evidence
- a prompt was sent to CLI when only a draft was saved
- a conversation exists without a conversation record
- Plan Builder output exists before it was generated
- Expected Shape exists before instantiator output
- initiation artifacts exist before they are created
- normal orchestration navigation is ready without a real target

Unsupported runtime behavior should be explicit and recoverable.

## Sequential Coordination Protocol

Use one active implementation path at a time.

1. Re-ingest this package and the current slice file.
2. Inspect current repo status and relevant files.
3. Decide the next single slice to execute.
4. Create one focused worker as its own visible Codex conversation/thread for that slice, unless the root explicitly chooses to do a tiny administrative edit itself.
5. Do not use hidden sub-agents for implementation workers. Workers must be inspectable conversations.
6. In the worker prompt, include the root orchestration thread id and require the worker to report back to that root thread when complete or blocked.
7. When the worker reports back, review the result before starting the next slice.
8. If the result is incomplete, re-prompt that worker conversation or create one correction worker conversation with the same report-back requirement.
9. Only move to the next slice after the current slice is accepted.

Follow the slice order unless the root finds a concrete reason to adjust it.

## Worker Report Requirements

Every worker or correction worker must report:

- status: complete or blocked
- summary of the change
- files changed
- tests or checks run
- known risks
- any behavior intentionally left unsupported
- exact next recommended action

The worker should send that report to the root orchestration conversation, not only leave it in the worker conversation. The worker should not declare the product objective complete. The root accepts or rejects the slice.

## Reasoning Guidance

The root thread should run on high reasoning.

Recommended worker reasoning:

- Slice 1 Agent Conversation Contract: high.
- Slice 2 Plan Builder Intake UI: high.
- Slice 3 Plan Builder Runtime Start And Stream: high, and xhigh only if runtime transport discovery becomes tangled.
- Slice 4 Plan Review/Re-Prompt/Instantiation Gate: high.
- Slice 5 Initiation Artifacts/Conversation Windows: high.
- Slice 6 End-To-End Usability Verification: medium for capture/check execution, high for final UX acceptance judgment.
- Mechanical formatting or narrow test corrections: low or medium.

Do not use higher reasoning as a substitute for smaller scope.

## Current Critical Path

The reusable conversation contract is first because later work depends on it. Without a shared conversation state and UI surface, the flow will keep encoding local, non-reusable assumptions in page code.

Runtime integration should not begin by pretending the CLI has conversation continuation semantics. The runtime start slice must verify what the local Codex surface actually supports and represent limitations honestly.

Expected Shape and initiation views are downstream of actual instantiator/initiation evidence.

## Review Checklist

For every slice, ask:

- Does any UI claim runtime activity without backend or runtime evidence?
- Did the worker preserve unsupported states honestly?
- Are reusable conversation components actually reusable?
- Are state transitions traceable to user input, backend response, persisted snapshot, runtime event, unsupported capability, or mock fixture?
- Are mock/demo fixtures visibly labeled?
- Did the worker avoid hardcoding future generated files as real artifacts?
- Did tests or stories cover the important state?
- Is the next slice genuinely unblocked?

## Recovery After Compaction

After compaction:

1. Read this file.
2. Read `overview.md`.
3. Read the active slice file.
4. Inspect `git status --short`.
5. Inspect recent worker reports.
6. Continue from the latest accepted slice, not from the beginning.

## Root Output Contract

When the root reports status, include:

- current objective
- accepted slices
- active worker or correction thread, if any
- current blocker or human decision, if any
- next single action
- files or thread ids needed for continuation
