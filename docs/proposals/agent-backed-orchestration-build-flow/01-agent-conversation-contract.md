# Slice 1: Agent Conversation Contract

## Goal

Define the reusable application and UI contract for agent conversations before wiring the Add Orchestration flow to real runtime behavior.

The target flow depends on one central primitive: an agent conversation that can be displayed, continued, summarized, and opened from multiple places in the app. This slice creates or refines that primitive without claiming that Plan Builder runtime execution is complete.

## Problem

The current Add Orchestration flow has a local `ConversationThreadView` and message list, but it is page-local and draft-oriented. It can display messages, but it is not yet a product-level representation of a live Codex agent conversation.

The current app also has domain records for conversations, task runs, events, raw event streams, and artifacts. Those concepts exist mostly through Open Tasks runtime paths, not as a reusable conversation surface for orchestration stages.

Without a shared contract, each orchestration screen will be tempted to invent its own version of "thinking", "running", "latest turn", "completed", and "current processing turn". That is exactly the product risk this proposal is meant to remove.

## Proposed Change

Introduce a reusable agent conversation contract that can represent:

- the conversation identity
- provider/runtime identity
- whether the conversation is interactive or read-only in the current view
- user turns
- assistant turns
- runtime/system event turns
- current in-flight turn, if any
- stream status
- latest known activity
- provenance for every visible state
- associated files and artifacts
- external thread id when known
- internal conversation id even when external runtime ids are absent
- unsupported or unavailable runtime state

The contract should distinguish:

- local user input
- optimistic UI placeholders
- backend acknowledgement
- first runtime event
- streamed runtime output
- completed runtime output
- failed runtime output
- unsupported integration

## Reusable UI Surfaces

Create or refine reusable UI components for:

- full agent conversation view
- read-only conversation window card
- current-turn indicator
- turn list
- file attachment strip
- artifact/event strip
- empty conversation state
- unsupported runtime state
- failed runtime state

The full conversation view should support interactive prompting when the current flow allows it.

The conversation window card should show only enough to scan:

- title
- role or stage
- latest turn/output summary
- current state
- whether work is active
- last updated timestamp

Clicking a conversation window should route to a full conversation view when the surrounding screen supports navigation.

## UX Requirements

- A conversation must never show "thinking" or "running" from local inference alone.
- If the app is waiting for the first runtime event, the UI should say that explicitly.
- If a backend accepted a request but no runtime event has arrived, that is not the same as the agent thinking.
- If the runtime is unsupported, the UI should say what did and did not happen.
- Every long-running operation must leave a visible current-action state.

## Evidence And Provenance

Every visible state should trace to one of these sources:

- user input
- local optimistic UI state
- backend response
- runtime event
- persisted snapshot
- unsupported capability
- mock/demo fixture

Mock/demo data must remain visibly labeled in Storybook and tests.

## Scope

In scope:

- Domain/application types for reusable conversation state.
- Reusable UI components or extraction of existing conversation UI.
- Storybook coverage for major states.
- Tests for state labeling and provenance.
- Migration path for Add Orchestration to consume this contract later.

Out of scope:

- Starting Plan Builder.
- Sending prompts to Codex CLI.
- Parsing every possible runtime event shape.
- Building initiation artifact views.
- Reworking the normal orchestration workspace.

## Acceptance Criteria

- There is a shared agent conversation state contract in the domain or application layer.
- There is a reusable full conversation view.
- There is a reusable read-only conversation window view.
- Existing Add Orchestration conversation UI can be mapped to the new contract without claiming runtime support.
- Storybook or equivalent isolated coverage shows idle, input-ready, starting, waiting for event, running, completed, failed, and unsupported states.
- Tests prevent "running" or "completed" labels from appearing without backend or runtime evidence.

## Root Decisions Before Delegation

The root orchestrator should decide:

- where the contract belongs in the current layer structure
- whether existing `ConversationThreadView` should be extracted directly or replaced by a new reusable component
- how much Storybook setup is stable enough to require in this slice

Do not decide the Codex runtime transport in this slice unless it is required to name the contract cleanly.
