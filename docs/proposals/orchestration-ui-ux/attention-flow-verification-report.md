# Attention-Flow Verification Report

Date: 2026-07-08

## Scope

This pass reviewed the Add Orchestration and build-initiation flow after the truthful state model, reusable orchestration UI components, client contract, Add Orchestration refactor, persisted drafts, stage-run evidence storage, and plan-builder route preflight work were in place.

Real plan-builder execution remains intentionally unwired. The verified production path stops at an explicit route-required blocker when no Open Task/worktree route is linked.

## States Reviewed

- First load and empty registry: `src/app/App.test.tsx` test `starts the orchestrations tab on a registry overview`.
- Empty Add Orchestration form: `src/app/App.tsx` `getAddOrchestrationCurrentAction` returns `Empty setup`.
- Partial form: `getAddOrchestrationCurrentAction` returns `Draft input in progress`.
- Valid local form: `getAddOrchestrationCurrentAction` returns `Ready to create local draft` and says plan-builder runtime has not started.
- File attachment before draft creation: `src/app/App.test.tsx` verifies uploaded files remain visible in the Add Orchestration conversation.
- Prompt submission with slow client response: `src/app/App.test.tsx` test `shows Add Orchestration draft, ready, and submitting states without claiming runtime work`.
- Draft creation failure: `src/app/App.test.tsx` test `shows a recoverable Add Orchestration failure tied to draft creation`.
- Persisted/local draft build workspace: `src/app/App.test.tsx` test `creates a build package before starting the live orchestration workspace`.
- Unsupported/generated-output slots: the same build-package test verifies expected outputs are `Not started` and not ready/completed.
- Runtime-event-backed running state: `src/app/App.test.tsx` test `shows plan-builder running only from a runtime-event-backed client response`.
- Root-startup readiness without action metadata: `src/app/App.test.tsx` test `does not infer Start Orchestration from a ready root-startup stage without client action metadata`.
- Local and Tauri route preflight: `src/infrastructure/localOrchestrationClient.test.ts` and `src/infrastructure/tauriOrchestrationClient.test.ts`.
- Storybook mock/demo state coverage: `src/ui/orchestration/OrchestrationComponents.stories.tsx`.

## Findings

No blocking attention-flow issue was found in the current supported product path.

The current flow can be narrated step by step:

1. I opened Orchestrations; the app loaded the registry and showed no registered orchestrations.
2. I opened Add Orchestration; the app asked for a title and prompt and did not imply runtime work.
3. I typed partial input; the app said draft input is in progress.
4. I completed the prompt; the app said the prompt is ready and plan-builder runtime has not started.
5. I submitted; the prompt appeared immediately as local user input, and the app showed it was submitting to the orchestration client.
6. When the client returned a draft, the build workspace showed a local/persisted draft, no plan-builder output, no generated files, and an explicit missing task/worktree route for plan-builder.
7. If draft creation fails, the error remains tied to draft creation and the prompt remains recoverable.
8. If an injected test client supplies runtime-event provenance, running copy appears. Local or unsupported paths cannot show running state.

## Truthfulness Checklist

- No UI path reviewed claims an agent is running without `backend_response` or `runtime_event` provenance.
- Local prompts are shown as user input or local draft state.
- Plan-builder output is absent unless supplied by a client fixture with explicit runtime/backend provenance.
- Expected package files remain expected/not-started slots; they are not marked ready or completed.
- `folderPath` is not treated as runnable `cwd`.
- Missing plan-builder route is represented as blocked/unsupported, and no hidden Open Task row is created.
- Storybook runtime examples are labeled as mock/demo, backend-response-shaped, or runtime-event-shaped fixtures.

## Evidence

Automated coverage:

- `src/app/App.test.tsx` covers registry first load, local draft creation, route-required build workspace copy, slow create-draft submission, create-draft failure, runtime-event-backed running, and omitted start action metadata.
- `src/infrastructure/localOrchestrationClient.test.ts` covers local draft truth, empty `stageRuns`, blocked plan-builder route metadata, no `cwd`/`taskId`/`worktreeId` fallback, and no synthesized completed backend stage.
- `src/infrastructure/tauriOrchestrationClient.test.ts` covers Tauri adapter payloads and blocked route-shaped client responses.
- `src-tauri/src/lib.rs` orchestration tests cover persisted draft reload, unsupported runtime requests, stage-run evidence rehydration without implied outputs, and rejection of runtime claims without backend/runtime provenance.
- `src/ui/orchestration/OrchestrationComponents.stories.tsx` covers mock/demo display states for statuses, current actions, stages, conversations, files, timelines, long content, and runtime-evidence-shaped examples.

Manual/code review:

- `src/app/App.tsx` current-action panels are present before submit, during submit, after draft creation, and after build-stage failures.
- Build-stage action buttons are omitted unless the client supplies an enabled supported action. Disabled unsupported actions are not presented as if they could run.
- Build conversation notes are local draft notes and do not claim to be sent to a backend role.
- Expected Shape copy says the outline is derived from local input and is not plan-builder output.

## Fixes Applied

No code or copy fix was needed during this Slice 6 pass. This report is the only Slice 6 artifact.

## Remaining Limitations

- Folder picker success/failure is covered by code review and fallback UI behavior, not a dedicated automated test in this pass.
- Reload/recovery is covered by the Tauri persistence tests and Rust draft reload tests, not an end-to-end Tauri UI run.
- Runtime acknowledgement, first runtime event, and completed event are represented only through injected test/Storybook fixtures unless future runtime integration supplies real backend/runtime events.
- Full visual screenshots were not captured in this pass; automated tests, Storybook build, and precise file/test references serve as the verification evidence.

## Conclusion

The current Add Orchestration path remains understandable and truthful while runtime integration is incomplete. The user can see what was accepted locally, what is persisted or held locally, why plan-builder cannot start, and what evidence would be required before the UI could show real running or completed orchestration work.
