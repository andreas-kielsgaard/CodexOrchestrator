# Slice 4: Add Orchestration Refactor

## Goal

Refactor the Add Orchestration flow so it uses the truthful state model, reusable UI components, and `OrchestrationClient` contract.

The user should be able to follow the flow moment by moment without being told that unfinished runtime behavior is happening.

## Problem

The current Add Orchestration flow combines setup, conversation, file upload, build-package creation, stage advancement, and live workspace transition in local component state.

That creates two UX issues:

1. Immediate actions are not always distinguished from real background work.
2. Simulated progress can appear as operational truth.

The user needs to know not just what has happened, but what is happening now.

## Proposed Change

Rebuild the Add Orchestration flow around explicit states:

- empty setup
- draft input
- valid and ready
- submitting prompt
- prompt accepted locally
- waiting for backend acknowledgement
- integration pending
- runtime event received
- failed

The flow should show a persistent current-action area that is honest and specific.

Example current-action copy:

- "Enter a title and prompt to create a draft."
- "Prompt ready. Plan-builder runtime has not started."
- "Submitting prompt to orchestration client."
- "Prompt accepted. Waiting for backend acknowledgement."
- "Plan-builder runtime integration is pending."
- "No plan-builder output yet."
- "Plan-builder is running." only when a real event says so.

## Scope

In scope:

- Replace local build progression with client-driven state.
- Use reusable components from Slice 2.
- Preserve file attachment and folder selection feedback.
- Add current-action and event/provenance indicators.
- Add tests around truthful state rendering.

Out of scope:

- Implementing real plan-builder execution.
- Implementing real instantiator execution.
- Creating root orchestration threads.
- Persisting all orchestration data unless Slice 5 has already done so.

## UX Requirements

Every user action must produce immediate feedback:

- Typing a title updates draft validity.
- Selecting a folder updates the selected folder or shows an error.
- Attaching files shows the files immediately.
- Submitting a prompt shows a local pending state immediately.
- Client success updates the state from the client response.
- Client failure shows a clear error and retry option.

Long-running or unsupported states must not be silent.

The flow should make clear which panel owns attention:

- Setup: title, folder, source context, files.
- Current action: what is happening now.
- Conversation: user input and real output or pending markers.
- Next step: what can be done next.

## Acceptance Criteria

- The Add Orchestration flow no longer advances through fake stage completion on local button clicks.
- The primary action label reflects the next real or supported action.
- Unsupported backend capability is represented as `integration_pending` or equivalent, not fake activity.
- User-submitted prompt appears immediately as local/user input.
- A pending request has a visible busy state.
- Errors are visible, recoverable, and tied to the action that failed.
- Tests cover draft, ready, submitting, integration-pending, failed, and real-running event states.

## Suggested Implementation Notes

Keep the first refactor focused on Add Orchestration. Do not try to clean up every orchestration workspace screen in the same slice.

If the backend is not ready, the default mock/local behavior should stop at a truthful pending/unsupported state. That is an acceptable product state during development.

Avoid labels such as "Build Package" if the package does not exist yet. Prefer "Draft package preview" or "Expected package shape" when showing design-only previews.
