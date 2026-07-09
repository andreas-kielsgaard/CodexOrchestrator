# Root Handoff: Agent-Backed Orchestration Build Flow

## Role

You are the root coordination thread for implementing the Agent-Backed Orchestration Build Flow proposal in the Codex Orchestrator repo.

Use high reasoning for root-level decisions.

## Start Here

Read:

- `docs/proposals/agent-backed-orchestration-build-flow/orchestrator-reingestion-package.md`
- `docs/proposals/agent-backed-orchestration-build-flow/overview.md`
- `docs/proposals/agent-backed-orchestration-build-flow/01-agent-conversation-contract.md`

Then inspect the current implementation enough to verify the first slice boundary:

- `src/app/App.tsx`
- `src/application/orchestrationClient.ts`
- `src/domain/orchestrationState.ts`
- `src/infrastructure/localOrchestrationClient.ts`
- `src/infrastructure/tauriOrchestrationClient.ts`
- `src-tauri/src/lib.rs`
- existing reusable UI under `src/ui/`
- existing Storybook setup under `.storybook/`

## Objective

Coordinate sequential implementation of the proposal so Add Orchestration becomes an agent-backed, stage-gated conversation flow.

The final target is not merely a clearer "integration pending" screen. The target is a real runtime-backed flow:

1. Plan Builder intake starts from user material, not a required title.
2. Submission starts or attempts to start a real agent conversation.
3. The UI shows runtime-backed conversation state.
4. Planner feedback continues the same conversation when supported.
5. Instantiation is started only after user approval.
6. Expected Shape appears only after instantiator evidence.
7. Initiation shows created artifacts and conversations.
8. Navigation to the normal orchestration view appears only after a real target exists.

## Non-Negotiable Rule

Do not fabricate orchestration facts.

If a runtime route, generated file, conversation, thread id, expected shape, initiation artifact, or navigation target does not exist, the UI must say that honestly.

## Coordination Protocol

Proceed one slice at a time.

1. Decide whether Slice 1 is still the correct first slice after inspecting the repo.
2. If it is, create a focused worker as its own visible Codex conversation/thread for Slice 1 Agent Conversation Contract.
3. Do not use hidden sub-agents for implementation workers. Workers must be inspectable conversations.
4. In the worker prompt, include this root thread id and require the worker to report back to this root thread when complete or blocked.
5. Review the worker result before moving on.
6. If the result needs correction, re-prompt that worker conversation or create one correction worker conversation.
7. Do not start Slice 2 until Slice 1 is accepted.

Keep the process sequential.

## Worker Prompt Requirements

Every worker prompt should include:

- proposal file for the slice
- root orchestration thread id for report-back
- root report-back requirement
- non-negotiable no-fabrication rule
- expected changed-file scope
- verification expectations
- requirement to avoid unrelated refactors
- requirement to preserve user/unrelated git changes

## First Slice Candidate

Slice 1: `docs/proposals/agent-backed-orchestration-build-flow/01-agent-conversation-contract.md`

Expected focus:

- reusable agent conversation contract
- reusable full conversation view
- reusable read-only conversation window
- state provenance rules
- Storybook or isolated state coverage where appropriate
- tests preventing running/completed claims without evidence

Do not implement Plan Builder runtime start in Slice 1 unless the root revises the slice boundary for a specific reason.

## Report Back

When you have launched or completed the first root action, report:

- current objective
- selected first slice
- worker thread id or reason no worker was launched
- expected completion signal
- any immediate blocker
