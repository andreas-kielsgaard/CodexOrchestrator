# Slice 6: Attention-Flow Verification

## Goal

Verify that the Add Orchestration and orchestration workspace flows are understandable while work is happening, not only after work completes.

This slice is a UX validation pass. It should be performed after the truthful state model, reusable components, client contract, Add Orchestration refactor, and initial runtime integration are in place.

## Problem

The core observed UX issue is attention flow. Users need to follow an agentic process over time. Static UI states are not enough. The app must continuously answer:

- What is happening now?
- Is the app working?
- Is an agent working?
- Is the app waiting for me?
- Is the app waiting for a backend response?
- Did something fail?
- Can I safely leave and come back?

Without this, users lose trust even if the screen has correct final states.

## Proposed Change

Run an explicit verification pass through the Add Orchestration flow and any connected runtime screens.

The pass should include:

- first-load state
- empty form
- partial form
- valid form
- folder picker success and failure
- file attachment
- prompt submission
- slow client response
- backend unsupported response
- backend error
- runtime acknowledgement
- first runtime event
- long-running event wait
- completed event
- reload/recovery if persistence exists

## Evaluation Checklist

For every user action:

- Is there immediate visual feedback?
- Does focus remain in a sensible place?
- Does the primary action update correctly?
- Is disabled state explained by surrounding context?
- Does the UI avoid inventing progress?
- Is pending work visibly pending?
- Are errors tied to the thing that failed?

For every long-running state:

- Is there a current-action message?
- Is the actor clear: user, app, backend, Codex, or unsupported integration?
- Is the state backed by real provenance?
- Is elapsed time shown when helpful?
- Is the next update source clear?
- Is there a retry/cancel action only if supported?

For every completed state:

- Is the completion backed by backend or runtime state?
- Are generated artifacts real?
- Is the next step clear?
- Is there a record of what happened?

## Scope

In scope:

- Manual UX review.
- Playwright or Testing Library interaction tests where practical.
- Storybook visual review of key states.
- Notes or screenshots for confusing states.
- Small copy and state-label corrections discovered during review.

Out of scope:

- Large refactors unless a blocker is found.
- Adding new runtime capabilities.
- Treating mock data as real validation.

## Acceptance Criteria

- A reviewer can narrate the flow in plain language at every step: "I did X; now the app is doing or waiting for Y."
- No action results in a silent wait.
- No screen claims real agent work without real state.
- Mock/demo states are clearly labeled.
- Slow and failed states are understandable.
- Reload behavior is clear, whether state is preserved or not.
- Findings are written up with screenshots or precise file/state references.

## Suggested Implementation Notes

Use Storybook for isolated states and the running app for the full flow. If the backend is still incomplete, include that as part of the verification: the UI should remain understandable even when it reaches an integration-pending stop.

This slice should be repeated after major orchestration runtime changes. Agentic workflows are especially sensitive to attention drift, so regression testing should include "what is happening now" checks, not only final-state assertions.
