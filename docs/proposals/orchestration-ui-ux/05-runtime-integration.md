# Slice 5: Runtime Integration

## Goal

Connect the orchestration UI/client contract to real persistence, backend commands, and runtime events incrementally.

This slice should make runtime facts real. It should not rely on UI simulation to imply progress.

## Problem

The current orchestration flow can show staged work, generated package files, and live orchestration views without those facts coming from a backend. That is useful for early design exploration but risky for product behavior.

The app needs backend-owned orchestration records and runtime events so the UI can honestly display progress.

## Proposed Change

Implement real orchestration client adapters behind the contract from Slice 3.

Potential backend capabilities, in rough order:

1. Persist orchestration drafts.
2. Load registered orchestrations and drafts.
3. Submit plan-builder prompt to a backend command or mark runtime support unavailable.
4. Store conversation/event records.
5. Record generated artifacts only after they are written.
6. Start root orchestration threads only when supported.
7. Stream or poll runtime events into the UI.

This should be implemented in narrow vertical increments. Each increment should return honest state when the next capability is not implemented.

## Scope

In scope:

- Add Tauri command definitions for supported orchestration actions.
- Add frontend Tauri adapter methods.
- Persist draft and event snapshots if persistence is ready.
- Surface backend errors and missing support explicitly.
- Update UI from backend snapshots/events.

Out of scope:

- Full orchestration automation if the product runtime is not ready.
- Guessing thread relationships before they are created.
- Marking generated files ready before they exist.
- Replacing all existing task runtime infrastructure.

## UX Requirements

Runtime integration must preserve the "what is happening now" contract:

- show when a request is being sent
- show when backend acknowledgement arrives
- show when runtime events start
- show the latest real output
- show elapsed time for active work when available
- show missing capability states honestly
- show retry/cancel only when supported

If the backend cannot perform a requested action, the UI should say that directly. Example:

"Plan-builder runtime is not connected yet. Your draft and prompt are saved, but no agent work has started."

## Acceptance Criteria

- Reloading the app preserves persisted orchestration draft state once persistence is implemented.
- UI "running" states come from backend acknowledgement or runtime events.
- Generated artifacts appear only after backend records say they exist.
- Unsupported runtime capabilities return explicit unsupported/integration-pending states.
- Client and Tauri adapter tests cover success, failure, and unsupported capability responses.
- The UI can recover from backend errors without losing the user's local prompt.

## Suggested Implementation Notes

Start with persistence before live execution. A durable draft is more valuable than a fake live stage.

Do not create placeholder files merely to make the UI look complete. If a future generated artifact is expected, represent it as "expected" or "not generated yet" until the backend writes it.

If event streaming is not ready, use polling or reloadable snapshots. The UI can still be honest by saying "Waiting for next update" rather than implying real-time streaming.
