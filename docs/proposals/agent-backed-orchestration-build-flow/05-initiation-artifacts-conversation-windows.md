# Slice 5: Initiation Artifacts And Conversation Windows

## Goal

Add the live initiation step after instantiation is approved.

The user should approve the instantiated plan, start orchestration initiation, watch the initiation work in the same agent-backed flow, inspect created artifacts and spawned conversations, and then navigate to the normal orchestration view.

## Problem

The current `start_orchestration` command returns a live-runtime-unavailable notice. It does not create root orchestration threads, record-root threads, startup prompts, files, or live orchestration snapshots.

The current UI can show live orchestration concepts, planner windows, work slices, and conversation-like surfaces. But those are not currently connected to the Add Orchestration build package as real initiation artifacts.

The target flow needs a bridge from "instantiation package approved" to "normal orchestration view is available".

## Proposed Change

After instantiation completes and Expected Shape is available:

1. Show an explicit action such as `Approve plan and initiate orchestration`.
2. On approval, send a fixed initiation prompt into the same agent conversation or into a verified supported runtime route.
3. Show Initiation as processing in the stage outline only after real start evidence exists.
4. Add an Initiation view beside Conversation and Expected Shape.
5. The Initiation view tracks artifacts and conversations as they are created.
6. When new conversations or root windows exist, render them as reusable read-only conversation window cards.
7. Clicking a window opens the relevant full conversation or normal workspace context when available.
8. When initiation completes, show a button to navigate to the normal orchestration view.

## Initiation Evidence

The initiation view should be built from real records, such as:

- files written
- startup prompts created
- root conversation/thread ids
- record conversation/thread ids
- orchestration home path
- repo locator files
- runtime events
- validation or startup checks
- unsupported or failed actions

The exact artifact list must come from implemented backend behavior. Do not hardcode files or conversations as complete before they exist.

## Conversation Windows

The reusable conversation window card from Slice 1 should be used here.

Each card should show:

- conversation title
- role, such as root orchestrator, record root, planner, worker, or initiation agent when known
- latest activity
- current state
- processing indicator if backed by evidence
- last updated time
- unavailable or unsupported state when relevant

The card is a preview, not an editable prompt box.

## Scope

In scope:

- Initiation stage action and state transitions.
- Initiation artifact view.
- Conversation window cards for created conversations.
- Navigation to normal orchestration view after initiation completes.
- Tests around unsupported, failed, processing, and completed initiation.

Out of scope:

- Implementing every possible root orchestration behavior if startup runtime is still incomplete.
- Making fake root threads for display.
- Replacing the normal orchestration view.

## UX Requirements

- The user should see what initiation is doing, not just that initiation was requested.
- If initiation creates files, show them as they are created.
- If initiation starts conversations, show them as windows.
- If initiation cannot start, say exactly what is missing.
- The navigation button should appear only when the normal orchestration view has a real target.

## Acceptance Criteria

- The initiation action is unavailable until instantiation output is approved.
- Initiation processing is backed by runtime/backend evidence.
- The Initiation view appears at the right time and stays hidden before it has meaningful content.
- Created artifacts are shown from backend/runtime records.
- Created conversation windows are shown from real conversation/thread records.
- The user can navigate to the normal orchestration view only after a live orchestration target exists.

## Root Decisions Before Delegation

The root orchestrator should decide:

- what concrete backend capability counts as initiation complete for the first increment
- whether root thread creation should use Codex thread tools, Codex CLI runtime, or another product-owned mechanism
- how much of the normal orchestration view must be wired before this slice can be accepted
