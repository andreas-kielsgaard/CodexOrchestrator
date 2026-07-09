# Slice 1: Truthful State Model

## Goal

Create a minimal orchestration state vocabulary that lets the UI be clear without pretending unfinished backend behavior exists.

This slice is the foundation for the rest of the proposal. It should not implement real orchestration runtime work. It should define how the UI talks about orchestration state while parts of the system are still draft, pending, unsupported, mocked, or waiting for events.

## Problem

The current flow can display confident operational language before the system has real evidence. A user can see messages such as "Running orchestration-plan-builder" or stage-completion copy even when the frontend is only advancing local state.

That creates a trust problem. The user cannot tell whether the app is:

- recording local input
- calling a backend command
- waiting on Codex
- showing placeholder design content
- showing real orchestration output

## Proposed Change

Define explicit status and provenance types for orchestration UI state.

Suggested high-level statuses:

- `draft`: local form or draft record exists, but no runtime action has started.
- `ready`: the user has provided enough input to start the next action.
- `starting`: a command has been requested and the app is awaiting acknowledgement.
- `waiting_for_event`: the app has started or requested work and is waiting for the first event.
- `running`: a real runtime event or backend acknowledgement confirms work is active.
- `blocked`: the flow cannot continue without user input, missing configuration, or a backend capability.
- `failed`: a command or load action failed.
- `completed`: a real action completed.
- `integration_pending`: the UI can represent this step, but runtime support is not implemented.
- `mock_preview`: Storybook, test, or demo fixture only.

Suggested provenance labels:

- `user_input`
- `local_draft`
- `persisted_snapshot`
- `backend_response`
- `runtime_event`
- `mock_fixture`
- `unsupported`

The UI should only show operational claims when provenance supports them. For example:

- A prompt typed by the user can be shown immediately as `user_input`.
- A local draft can say "Draft saved locally" if persistence exists, or "Draft held in this session" if it does not.
- A stage can say "Ready to run plan builder" before backend support exists.
- A runtime step can say "Running" only after a backend response or runtime event confirms it.

## Scope

In scope:

- Define TypeScript types for orchestration statuses and provenance.
- Replace simulated operational copy with honest draft, ready, pending, or unsupported copy.
- Add helper functions for deriving display labels from status plus provenance.
- Add tests for label derivation and state transitions that must not invent runtime progress.

Out of scope:

- Persisting orchestration records.
- Starting real Codex plan-builder runs.
- Creating real generated files.
- Creating Codex threads.
- Streaming runtime events.

## UX Requirements

Every status must answer a user's immediate question:

- What happened?
- What is happening?
- What is the app waiting for?
- What can I do next?

Recommended copy examples:

- "Draft ready"
- "Prompt accepted locally"
- "Ready to start plan builder"
- "Backend integration pending"
- "Waiting for runtime acknowledgement"
- "No plan-builder output yet"
- "Mock preview"

Avoid copy such as:

- "Running orchestration-plan-builder" unless a real command/event confirms it.
- "Generated files ready" unless files exist or a backend snapshot says they are ready.
- "Root startup prepared" unless root startup actually completed.

## Acceptance Criteria

- No UI state claims an orchestration agent is running unless the state is backed by `backend_response` or `runtime_event`.
- No generated file is marked ready or complete unless backed by real state.
- Mock/demo states are visibly labeled as mock/demo when used outside tests.
- Tests cover that local button clicks cannot advance a stage into real runtime states by themselves.
- The Add Orchestration flow can represent incomplete backend support without appearing broken or silent.

## Suggested Implementation Notes

Start by moving orchestration status types out of `App.tsx` into a domain or application module. Keep the first version intentionally small. The goal is not to model every future orchestration detail. The goal is to prevent misleading current behavior.

This slice should also introduce a small display helper layer, for example:

- `getOrchestrationStatusLabel(state)`
- `getOrchestrationStatusDescription(state)`
- `canShowRuntimeProgress(state)`
- `isMockOrUnsupported(state)`

These helpers will later feed reusable UI components and Storybook states.
