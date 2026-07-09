# Orchestration Build Flow Usability Audit

Date: 2026-07-08

## Scope

User-review usability pass for the Codex Orchestrator Add Orchestration / orchestration build flow, with emphasis on immediate feedback, attention flow, and whether the user can understand what is happening while work is in progress.

## Evidence Captured

- `01-initial-dashboard.png`: Browser-run app at `http://127.0.0.1:5173/`.

## Result

The flow could not be reached in the live application during this pass.

The browser build immediately shows a centered error card:

- Title: `Backend unavailable`
- Detail: `Cannot read properties of undefined (reading 'invoke')`
- Only visible action: `Retry`

This is a blocking usability issue because a user trying the local app cannot reach the Orchestrations tab or the Add Orchestration flow. The error does not explain whether they are in the wrong runtime, whether Tauri is required, or what they should do next besides retrying.

The Tauri desktop app did launch, but the available desktop capture/control layer could not resolve the window handle after launch, so no valid screenshot-backed pass through the desktop flow was possible in this run.

## Step List

1. Open local browser app.
   - Health: blocked.
   - The app loads an error-only state before the product shell appears.
   - The message is technically specific but not user-actionable.

2. Attempt to continue with Retry.
   - Health: not completed.
   - Retry is visible, but because the underlying missing `invoke` path is structural, it is unlikely to repair the state.

3. Attempt to audit the Tauri desktop app.
   - Health: blocked by capture/tooling.
   - Tauri compiled and launched `Codex Orchestrator`, but a valid screenshot/control handle could not be maintained for the audit.

## UX Findings

1. Browser/dev entry has no graceful fallback.
   - In `src/main.tsx`, orchestration switches to a local client outside Tauri, but task dashboard and runtime clients remain Tauri clients. The user experiences this as a backend outage rather than a runtime-mode mismatch.
   - Recommendation: either provide local/dev clients for the full app shell or show an explicit "Desktop runtime required" state with a command/link-like instruction for launching Tauri.

2. The startup error is too generic for a product flow.
   - The error card has strong visual focus, but it does not answer: what happened, what is happening now, what can I do, or whether retry is expected to work.
   - Recommendation: distinguish `starting`, `retrying`, `wrong runtime`, and `backend unavailable` states. Retry should show immediate busy feedback and a short current action.

3. The flow cannot be evaluated until entry is reliable.
   - Because the Add Orchestration flow is behind app startup, the most important usability requirement is a dependable path into the flow.
   - Recommendation: treat startup reachability as Slice 0 before further UX polish. A beautiful in-flow progress model will still fail the user if the first screen cannot get them there.

## Accessibility Risks

1. Error recovery depends on a single generic button.
   - From the screenshot, there is no secondary help text or next-step explanation for assistive technology users.

2. Runtime-mode errors may be announced as backend failures.
   - If the real issue is "this browser mode cannot use Tauri invoke," the accessible message should say that plainly rather than exposing a JavaScript exception.

## Evidence Limits

- This audit did not reach the Orchestrations tab or Add Orchestration form in the live app.
- This audit cannot claim anything about the in-flow progress states from current live evidence.
- Prior automated tests and Storybook builds were not used as substitutes for live UX evidence.

## Recommended Next Pass

After the app has a reachable local-browser flow or the Tauri desktop window can be captured reliably, rerun the usability pass through these states:

1. Orchestrations tab entry and empty/registry overview.
2. Add Orchestration form before input.
3. Prompt entered and ready-to-submit state.
4. Submit click immediate feedback within 100 ms.
5. In-flight draft creation / plan-builder pending state.
6. Created build package detail view.
7. Request build-stage action.
8. Unsupported/integration-pending response.
9. Failed createDraft response.
10. Navigation away/back with persisted draft state.
