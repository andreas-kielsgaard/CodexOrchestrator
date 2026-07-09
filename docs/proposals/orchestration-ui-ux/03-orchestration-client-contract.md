# Slice 3: Orchestration Client Contract

## Goal

Introduce a frontend-facing orchestration application boundary so the UI stops creating orchestration lifecycle facts locally.

This slice defines the contract. It does not need to implement the real runtime yet.

## Problem

The task dashboard already has an application client boundary. The orchestration flow does not. It currently creates build packages, advances stages, and starts draft live views through local helper functions inside `App.tsx`.

That makes it too easy for UI code to invent state. It also makes it hard to later swap in Tauri-backed persistence, Codex runtime events, or Storybook mocks cleanly.

## Proposed Change

Add an `OrchestrationClient` interface in the application layer.

Suggested methods:

- `loadOrchestrations(): Promise<OrchestrationDashboardSnapshot>`
- `createDraft(input): Promise<OrchestrationDraftSnapshot>`
- `submitPlanBuilderPrompt(input): Promise<OrchestrationActionResult>`
- `loadOrchestration(id): Promise<OrchestrationSnapshot>`
- `cancelDraft(id): Promise<OrchestrationActionResult>`
- `subscribeToOrchestrationEvents?(id, listener): Unsubscribe`

The exact method list can be smaller for the first pass. The important part is that the UI asks a client for state rather than manufacturing lifecycle truth.

## Required Contract Properties

Each returned state should include:

- a stable id if one exists
- status
- provenance
- user-visible current action
- last updated time if known
- whether the state is persisted
- whether runtime support is available
- errors, blockers, or missing capability notices

The contract must support honest incomplete states. For example, an early mock/local client can return:

- status: `integration_pending`
- provenance: `unsupported`
- currentAction: `Plan-builder runtime is not connected yet.`

That is better than returning a fake running state.

## Scope

In scope:

- Define TypeScript interfaces and DTOs.
- Add a local/mock implementation for tests and Storybook.
- Add an adapter placeholder for future Tauri implementation.
- Update the app composition so orchestration screens receive an `OrchestrationClient`.

Out of scope:

- Real Tauri persistence.
- Real Codex thread creation.
- Real event streaming.
- Real generated file writes.

## UX Requirements

The client contract should be able to answer:

- Can the user start this action?
- Did the app accept the action?
- Is runtime support available?
- Is the app waiting for a response?
- Is there real output yet?
- What should the user do next?

These answers should not be derived from optimistic UI alone unless clearly marked as local pending state.

## Acceptance Criteria

- Orchestration screens no longer call local helper functions to create fake completed stages.
- The app can run with a mock orchestration client in tests and Storybook.
- The mock client identifies itself through mock/demo provenance.
- The UI can display unsupported or integration-pending states cleanly.
- Tests confirm client responses drive visible orchestration state.

## Suggested Implementation Notes

Mirror the existing task client pattern where useful, but do not force orchestration into the task model. Orchestration has different needs: long-running state, multiple actors, generated artifacts, thread relationships, blockers, and event streams.

Keep the first interface narrow. It is acceptable to support only draft creation and prompt submission initially, as long as the interface is honest about unsupported runtime work.
