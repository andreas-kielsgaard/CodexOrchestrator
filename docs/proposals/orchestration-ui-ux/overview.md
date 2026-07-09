# Orchestration UI/UX Change Proposal

## Purpose

This proposal documents a sequenced set of UI, UX, and frontend architecture slices for the Add Orchestration and orchestration workspace flows. The orchestration product surface is still in development, so the central design constraint is truthfulness: the UI must not imply that an agent, backend command, generated file, or Codex thread exists until the app has a real source of truth for that fact.

The near-term goal is to make the flow easier to follow without inventing progress. The user should always understand:

- what they just did
- what the app accepted locally
- what the app is currently attempting
- what the app is waiting for
- which information is real, pending, mocked, or not implemented yet

## Current Observations

The current Add Orchestration flow already has useful product ideas: a plan-builder conversation, a build package concept, a stage list, file attachments, plan review, and a live orchestration workspace. However, much of this behavior is currently represented in local React state and helper functions inside `src/app/App.tsx`.

The flow can therefore look more complete than it is. For example, UI copy can say that orchestration-plan-builder is running, that an instantiation package is prepared, or that root startup prompts are ready even though the current frontend path is not receiving those facts from a real orchestration backend or event stream.

This is the main UX risk. The issue is not only component polish or loading states. It is attention flow: the app needs to tell the user what is happening now, and that statement needs to be grounded in real state.

## Non-Negotiable Design Rule

Do not make up orchestration details ahead of time.

The UI must not fabricate:

- generated file names as completed outputs
- thread IDs
- Codex agent actions
- plan-builder output
- instantiator output
- root startup completion
- live run status
- stage completion
- elapsed work that is not happening

While the backend or orchestration runtime is incomplete, the UI should use honest states such as:

- Draft created
- Ready to run
- Backend integration pending
- Waiting for first event
- No output yet
- Not started
- Unable to start
- Mock preview

Any mock/demo fixture used for Storybook, tests, or local design exploration must be clearly labeled as mock/demo data and must not be confused with runtime truth.

## Proposed Sequence

1. [Truthful State Model](./01-truthful-state-model.md)
2. [Reusable UI and Storybook Foundation](./02-reusable-ui-storybook-foundation.md)
3. [Orchestration Client Contract](./03-orchestration-client-contract.md)
4. [Add Orchestration Refactor](./04-add-orchestration-refactor.md)
5. [Runtime Integration](./05-runtime-integration.md)
6. [Attention-Flow Verification](./06-attention-flow-verification.md)

This order intentionally separates UI truthfulness from runtime completeness. The first slices make the app safe and testable while behavior is still incomplete. Later slices add real persistence, events, and backend orchestration commands.

## Why This Sequencing

The truthful state model comes first because every other slice depends on shared language for incomplete, pending, and real states.

Reusable UI and Storybook come second because the app needs isolated design-state coverage before the flow grows more complex. Storybook should show truthful states, not imaginary orchestration success.

The orchestration client contract comes third because the UI needs an application boundary before it can stop manufacturing lifecycle facts locally.

The Add Orchestration refactor comes fourth because it should be rebuilt using the new state vocabulary, reusable components, and client contract.

Runtime integration comes fifth because real backend commands and event streams should be added into an interface that already knows how to represent waiting, failure, and missing support honestly.

Attention-flow verification comes sixth because the final proof is experiential: a user should be able to follow the flow moment by moment and understand what the app is doing.

## Target Architecture Direction

The intended direction is:

- `src/domain` owns orchestration state types and pure transition rules.
- `src/application` owns an `OrchestrationClient` interface and application-facing DTOs.
- `src/infrastructure` owns Tauri, local/mock, and future runtime adapters.
- `src/ui` or `src/app/components` owns reusable UI primitives and orchestration display components.
- `src/app` composes screens from clients, state, and UI components.
- Storybook mounts components and flows with explicit mock providers.

## Success Criteria For The Whole Proposal

- Add Orchestration no longer displays simulated progress as real progress.
- Every visible status can be traced to local user input, a client response, a persisted snapshot, or a runtime event.
- The user gets immediate feedback after every action.
- Long-running work has a visible current action, waiting state, or error state.
- Reusable UI elements are extracted enough to support consistent buttons, status pills, tabs, stages, files, conversations, and timelines.
- Storybook can render the major flow states without Tauri.
- The runtime boundary is ready for real orchestration commands without forcing the UI to invent missing details.
