# Agent-Backed Orchestration Build Flow Proposal

## Purpose

This proposal defines the target Add Orchestration build flow for Codex Orchestrator.

The current implementation can capture and persist orchestration drafts, but it does not yet start a real Plan Builder agent, stream the agent's work, continue an agent conversation, run the instantiator, or initiate live orchestration roots. The target product behavior is an agent-backed, stage-gated conversation flow where the user can always understand both:

- what has happened
- what is happening now

The proposal is intentionally sequential.

## Current State

The Add Orchestration flow currently behaves as a truthful draft airlock.

Observed implementation facts:

- `src/app/App.tsx` calls `orchestrationClient.createDraft(...)` when the user submits the Add Orchestration form.
- `src/infrastructure/tauriOrchestrationClient.ts` maps that to the Tauri command `create_orchestration_draft`.
- `src-tauri/src/lib.rs` persists a draft snapshot through `create_orchestration_draft_record`.
- The persisted snapshot includes `stageRuns: []`, blocked runtime routes, expected output placeholders, and messages that no plan-builder output or Codex thread exists yet.
- The existing Open Tasks runtime path can start Codex task runs through `start_codex_task_run`, but the Add Orchestration path does not call it.
- The current Tauri `SystemCodexCommandRunner` uses process completion output, not a live UI stream.
- The UI exposes later-stage concepts such as `Expected Shape` and expected output slots before the instantiator has produced them.
- Re-prompting in the build screen currently adds a local draft note rather than sending a prompt into the same agent conversation.

That is not only a visual polish issue. The runtime behavior behind the intended flow is missing.

## Target User Flow

1. The user clicks Add Orchestration.
2. The UI opens a Plan Builder intake screen. The screen is only about building the prompt and attaching materials. It does not require a title yet, does not show future stage detail, and does not imply processing before anything is submitted.
3. The user pastes text or uploads materials and submits.
4. The app forwards the input to a new Codex agent conversation through a real supported runtime path. The UI shows the conversation stream from that runtime, including starting, waiting for first event, running, output, failure, or completion.
5. The user remains in the same conversation view. The left flow outline shows Plan Builder as processing only while runtime evidence supports that state.
6. When the Plan Builder response completes, the user can review the output, re-prompt the same conversation with feedback, or confirm the plan and start instantiation.
7. Instantiation continues the same conversation with a fixed prompt to instantiate the approved plan. The UI shows Instantiator processing and does not expose Expected Shape until the instantiator has produced the relevant material.
8. After instantiation completes, Expected Shape becomes available. The user can review or re-prompt before approving live orchestration initiation.
9. Initiation continues the same conversation with a fixed prompt to start the orchestration root flow. A new initiation view tracks created artifacts and any conversations or windows created during startup.
10. When initiation completes, the user can navigate to the normal orchestration view.

## Non-Negotiable Product Rule

Do not make up orchestration details ahead of time.

The UI must not claim or imply:

- an agent is thinking unless the app has a real runtime signal
- a prompt was sent to a CLI agent unless a backend command actually sent it
- a conversation exists unless the app has created or loaded a conversation record
- a generated file exists unless a backend or runtime record says it exists
- expected shape exists before the instantiator produced it
- root startup happened before initiation created the relevant artifacts or conversations

Unsupported or unfinished behavior must be shown as unsupported, not as pending work.

## Proposed Sequence

1. [Agent Conversation Contract](./01-agent-conversation-contract.md)
2. [Plan Builder Intake UI](./02-plan-builder-intake-ui.md)
3. [Plan Builder Runtime Start And Stream](./03-plan-builder-runtime-start-and-stream.md)
4. [Plan Review, Re-Prompt, And Instantiation Gate](./04-plan-review-reprompt-instantiation.md)
5. [Initiation Artifacts And Conversation Windows](./05-initiation-artifacts-conversation-windows.md)
6. [End-To-End Usability Verification](./06-end-to-end-usability-verification.md)

This order keeps the work honest. First define reusable conversation state and UI. Then make the intake flow truthful. Then connect real runtime behavior. Only after Plan Builder output is real should the flow expose instantiation, expected shape, initiation artifacts, and navigation into the normal orchestration workspace.

## Target Architecture Direction

- `src/domain` should own durable state types and pure transition rules for agent conversations, orchestration stages, and provenance.
- `src/application` should own client contracts for orchestration build stages, agent conversation state, runtime submission, and evidence snapshots.
- `src/infrastructure` should own Tauri, browser-dev, Codex CLI, app-server, or local runtime adapters.
- `src/ui` should own reusable conversation views, conversation window cards, stage indicators, status pills, evidence labels, and file/artifact lists.
- `src/app` should compose the Add Orchestration flow from reusable state-aware components rather than encoding all behavior inline.

## Success Criteria

- The first Add Orchestration screen is an intake screen, not a full build package screen.
- The app does not require a user-supplied title before Plan Builder work starts.
- The Plan Builder conversation view is reusable across the app.
- The same reusable conversation view can show idle, starting, waiting for first runtime event, running, completed, failed, and unsupported states.
- The UI shows active work only when backed by runtime evidence.
- User feedback to the planner continues the same conversation.
- Instantiation is a stage transition, not a separate hidden flow.
- Expected Shape is unavailable until instantiator output exists.
- Initiation artifacts and created conversations are visible as they are created.
- Users can always tell what is happening now.

## Out Of Scope

- Pretending the CLI supports a conversation object before that behavior is verified.
- Committing to `codex exec`, `codex app-server`, or another runtime path before the runtime contract slice verifies what the app can actually support.
- Replacing the normal orchestration workspace beyond the entry point needed after initiation succeeds.
