# Slice 3: Plan Builder Runtime Start And Stream

## Goal

Connect Plan Builder submission to a real Codex agent runtime path and stream or otherwise surface real runtime events into the reusable conversation view.

This is the slice where the product begins doing the thing the user expects: submit source material to an agent using the Orchestrator Plan Builder skill and show the agent's actual work.

## Problem

The current Add Orchestration path creates a draft. It does not start a Codex CLI process for Plan Builder.

The app has existing runtime infrastructure for Open Tasks, including `start_codex_task_run`, task runs, conversations, events, artifacts, and raw Codex JSONL capture. It also has Codex runtime/parser work in TypeScript and app-server message parsing. However, the orchestration build flow is not wired to those runtime paths, and the existing Tauri command path waits for process completion rather than streaming live UI updates.

The user expectation is not just final output. The user expects to see the agent doing work.

## Proposed Change

Add a supported orchestration Plan Builder runtime start path.

The implementation should verify the actual Codex runtime surface before committing to transport details. Possible routes include:

- reusing the existing task-run runtime command if it can represent the orchestration conversation correctly
- adding an orchestration-specific Tauri command that creates an orchestration stage run and starts Codex
- using a `codex app-server` based session if that is the correct way to preserve and continue conversations
- using `codex exec --json` only if the limitations are explicit and acceptable for the current increment

The selected route must support, or honestly degrade around:

- starting a new agent conversation
- preserving an internal conversation id
- storing external runtime thread ids when available
- associating prompts, outputs, raw streams, events, and artifacts with the orchestration draft/stage
- showing start acknowledgement
- showing first runtime event
- showing running/current activity
- showing terminal completion or failure
- continuing the same conversation later, if supported

If true continuation is not supported by the runtime, the app should not claim it is continuing context. It should represent the limitation plainly.

## Plan Builder Prompt

The runtime prompt should use the Orchestrator Plan Builder skill intentionally.

The prompt should include:

- the user's pasted source material
- attached file references or contents according to the storage/runtime design
- any selected working directory or orchestration home facts that are truly known
- explicit instruction not to instantiate or start root threads during Plan Builder
- explicit instruction to produce plan output suitable for the next approval gate

Do not hardcode a guessed final prompt without reviewing the local `orchestration-plan-builder` skill and current runtime constraints.

## Runtime State Model

The flow should distinguish:

- `not_started`: no prompt submitted to runtime
- `starting`: app requested backend/runtime start
- `waiting_for_first_event`: backend acknowledged but no runtime event has arrived
- `running`: runtime event confirms active work
- `completed`: terminal runtime event or backend result confirms completion
- `failed`: runtime command or terminal event failed
- `unsupported`: runtime route cannot start

These states should appear in the left flow outline, current-action panel, and conversation view consistently.

## Scope

In scope:

- Add or adapt backend command(s) needed to start Plan Builder.
- Add frontend client contract methods for Plan Builder start/continue.
- Persist stage-run evidence for prompts, output, raw stream, events, and conversation ids.
- Feed runtime events into the reusable conversation view.
- Add tests around unsupported, start failure, running, completion, and raw event persistence.

Out of scope:

- Instantiator prompt flow.
- Initiation/root startup flow.
- Full scheduler or multi-agent orchestration.
- Pretending long-running streaming works if the runtime only provides a final response.

## UX Requirements

- The first visible confirmation after submit must be grounded in actual action.
- "The prompt was passed to an agent" should be shown only after the backend has actually started or accepted the runtime request.
- "Agent is thinking" should appear only after runtime evidence supports active work.
- If only a final-response runtime is available in the current increment, the UI should show "waiting for agent response" instead of a fake live stream.
- The user should never be stranded on a screen with no explanation of whether work is active.

## Acceptance Criteria

- Submitting Plan Builder material starts a real backend/runtime path or returns an explicit unsupported state.
- The Add Orchestration UI no longer treats draft persistence as Plan Builder execution.
- Runtime events or final output are visible in the reusable conversation view.
- Stage state reflects runtime evidence.
- Completion shows the Plan Builder output in the same conversation view.
- Failure shows what failed and whether the input was preserved.
- Reloading the draft can recover enough conversation/stage-run state to avoid losing the user's place.

## Root Decisions Before Delegation

The root orchestrator should decide:

- whether to base the first increment on existing Open Tasks task-run infrastructure or a new orchestration-specific command
- whether live streaming is required for the first runtime increment or whether an honest "waiting for final response" state is acceptable temporarily
- what storage route is required for attached materials before sending them to Codex
