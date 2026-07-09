# Orchestration Build Flow Usability Audit - Live Pass

Date: 2026-07-08

## Scope

Hands-on user review of the Add Orchestration / orchestration build flow in the live browser dev app after fixing the browser-runtime startup blocker.

Primary lens:

- Can the user tell what has happened?
- Can the user tell what is happening right now?
- Does every action give immediate feedback?
- Does the flow avoid claiming runtime work that has not happened?

## Technical Fixes Made First

The browser app previously blocked at startup with:

```text
Backend unavailable
Cannot read properties of undefined (reading 'invoke')
```

Fixes:

- Added `src/infrastructure/browserDevClients.ts`, a browser-only dev client bundle for the task dashboard, task detail, and runtime command surfaces.
- Updated `src/main.tsx` so non-Tauri browser mode uses those browser dev clients while Tauri mode keeps the real Tauri clients.
- Updated `src/app/App.tsx` so `selectOrchestrationDirectory` does not call Tauri `invoke` outside Tauri.

Verification before live pass:

```text
npm run build
npm run lint
```

Both passed.

## Evidence

Screenshots are saved in this folder:

- `01-app-entry.png`
- `02-orchestrations-overview.png`
- `03-add-orchestration-form.png`
- `04-form-ready.png`
- `05-after-submit-immediate.png`
- `06-build-package-created.png`
- `07-build-package-scrolled.png`
- `08-local-note-ready.png`
- `08b-local-note-ready-viewport.png`
- `09-local-note-submitted-immediate.png`
- `10-local-note-added.png`

## Step Review

1. App entry.
   - Health: improved.
   - The browser app now reaches the shell instead of showing the backend error.
   - Remaining UX issue: the default screen is Open Tasks, even though this review task is orchestration-oriented. The Orchestrations navigation item is visible and clear.

2. Orchestrations overview.
   - Health: usable.
   - Empty state clearly says no orchestrations are registered.
   - The main Add Orchestration action is present.
   - Risk: the empty-state CTA sits low in the panel and was partially near the viewport edge in the captured viewport. It is discoverable, but not as strong as it could be.

3. Add Orchestration form.
   - Health: mostly good.
   - The right side immediately explains the current state: `Empty setup`, then `Ready to create local draft`.
   - The copy is unusually honest and helpful: `Local draft only`, `Plan-builder runtime has not started`, and `User input held locally`.
   - Risk: the title field clipped the long title visually. The input is functional, but the user cannot easily inspect the full name without cursoring.

4. Ready-to-submit state.
   - Health: strong.
   - After title and prompt entry, the primary button enables and the conversation/state panel changes from empty setup to ready.
   - This does a good job separating "prompt accepted locally" from "plan builder is running."

5. Submit local draft.
   - Health: mixed.
   - The draft is created successfully and the app transitions to a build package view.
   - Because the local client resolves very quickly, there was no durable observable in-flight state in the live pass. Automated tests cover `Submitting prompt`, but a real user on a fast path may only see the completed transition.
   - Recommendation: show a short-lived acknowledgement or persistent activity row after submission, such as `Draft created from local input`, so the user has a clear causal bridge from click to new workspace.

6. Build package workspace.
   - Health: good for truthfulness, mixed for actionability.
   - The page strongly communicates `Backend integration pending` and `Unsupported integration`.
   - The top summary says no Codex run can start because no explicit task/worktree route is linked.
   - Generated files are labelled `Not started`, which avoids making up outputs.
   - Risk: there is no obvious next action for plan-builder. The page explains why work cannot run, but the user may still ask, "What can I do next?"

7. Stage/action model.
   - Health: blocked by design, likely intentionally.
   - Implementation suppresses the `requestBuildStage` action when the current stage is unsupported.
   - This is truthful, but it leaves the user with a status board and no remediation path.
   - Recommendation: add an explicit disabled action or remediation panel: `Connect a runtime route to enable Plan Builder`, with exact missing prerequisites.

8. Local note action.
   - Health: visually present, weakly targetable.
   - The note input and yellow icon-only submit button appear in the conversation panel.
   - Browser-visible DOM did not reliably expose the submit button once focus was in the textarea, and the button has no visible text.
   - Recommendation: give the icon-only submit button a visible tooltip/label state and ensure its accessible name is stable. Consider a text label such as `Add note` in this dense workflow.

## Strengths

- The flow is careful not to fabricate runtime progress.
- State labels are specific: `User input held locally`, `Backend integration pending`, `Not started`.
- The conversation model helps users understand what input has been captured.
- The build package view gives a useful stage map and expected-output list without pretending files exist.

## UX Risks

1. Missing causal bridge after submit.
   - The transition from form to build workspace is fast, but a user may not see "working" feedback.
   - The resulting page is clear after the fact, but not always clear during the transition.

2. Truthful blocked state lacks remediation.
   - The app says why plan-builder cannot run, but does not provide a next setup action.
   - This protects truthfulness but weakens forward momentum.

3. Full-page capture exposed repeated sticky/sidebar content.
   - The layout appears to duplicate top/sidebar content in full-page screenshots, likely from sticky positioning during full-page capture.
   - This may not affect normal viewport use, but it is a visual QA smell worth checking across screenshot tools and long-page scrolling.

4. Icon-only local note submit is too opaque.
   - The pencil icon reads like edit rather than send/add.
   - The action was harder to target than expected in the browser-control accessibility surface.

## Accessibility Risks

- Icon-only controls need stable accessible names and visible affordances. The local note submit control should be easier to identify and activate.
- Long status phrases inside pills can wrap awkwardly, for example `Unsupported` splitting across lines in the stage list.
- The title input clips long names without a preview or wrapping display until after submit.
- The blocked runtime state should be announced as a clear status plus remediation, not only as repeated warning pills.

## Recommendations

1. Add persistent activity confirmation after draft creation.
   - Example: `Draft created locally from your prompt. Plan-builder has not started.`

2. Add a remediation panel for integration-pending stages.
   - Example: `To run Plan Builder, link a task/worktree runtime route.`

3. Make unavailable actions explicit.
   - Show disabled `Start Plan Builder` with reason instead of hiding the action entirely.

4. Improve local note submit.
   - Use `Add note` text or icon plus text.
   - Keep the icon if desired, but do not rely on the pencil alone.

5. Check long-page/sticky rendering.
   - Full-page screenshots show repeated header/sidebar bands. Verify whether this is capture-only or a real scroll/reflow issue.

## Evidence Limits

- This pass used browser dev mode with local in-memory orchestration behavior, not the Tauri persistence backend.
- The local create-draft operation resolves too quickly to evaluate long-running spinner behavior naturally.
- Browser accessibility snapshots were imperfect for the local note submit button, so that finding should be verified with keyboard and screen-reader checks.
