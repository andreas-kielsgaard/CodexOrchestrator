# Slice 2: Plan Builder Intake UI

## Goal

Refactor the first Add Orchestration screen into a Plan Builder intake screen.

The user should arrive at a focused prompt-building experience. They should be able to paste text, attach materials, and submit the material to Plan Builder. The screen should not require a title yet and should not display future-stage details that are not available.

## Problem

The current Add Orchestration form asks for a title and folder before the user can create a draft. After submit, the user lands in a build package view with future steps, expected outputs, and an Expected Shape tab even though no Plan Builder agent has run.

That is truthful in parts, but it is not the intended experience. It also makes "integration pending" feel like the app might be working in the background, when the current implementation means "not wired".

## Proposed Change

Make Add Orchestration step 1 a dedicated Plan Builder intake view:

- no required title field before first submit
- no required folder decision before first submit unless the runtime route truly needs it
- upload and paste controls centered around source material
- one visible stage: Plan Builder
- Plan Builder marked not started before submit
- Plan Builder marked starting/running only after real start evidence exists
- no Expected Shape tab
- no instantiation package panel
- no future stage details

The UI may show the broader process in lightweight copy or as a compact outline only if it does not imply future steps are already available. The primary screen should remain the conversation input.

## Title Behavior

The user should not have to name the orchestration before Plan Builder runs.

Possible title sources, to be decided during implementation:

- generated from the first Plan Builder output
- derived from the first meaningful input line
- requested at the review/approval step
- editable after Plan Builder output exists

The implementation must not silently invent a durable title if that title becomes user-visible as a product fact. If an interim title is needed for storage, label it as an internal draft title.

## Submission Behavior

When the user submits source material, the app should provide immediate feedback:

- local input accepted
- request being sent
- backend acknowledged, if applicable
- waiting for first runtime event, if applicable
- runtime unsupported, if applicable
- start failed, if applicable

If real runtime start is not implemented yet, the UI should not navigate into a pretend build package. It should stay in the intake/conversation surface and show a clear blocked state.

## Scope

In scope:

- Refactor Add Orchestration screen structure.
- Remove required user-facing title from the initial submit path.
- Hide future stage details and Expected Shape before real outputs exist.
- Use the reusable conversation components from Slice 1.
- Preserve file upload and pasted text.
- Preserve draft persistence where safe, but label it accurately.

Out of scope:

- Real CLI agent start unless Slice 3 has already made it available.
- Instantiator behavior.
- Initiation artifacts.
- Normal orchestration workspace navigation.

## UX Requirements

- The first screen should answer: "What should I give Plan Builder?"
- The left stage indicator should answer: "Plan Builder has not started yet" before submit.
- The submit button should describe the actual action.
- If the button only saves a draft, it must say so.
- If the button starts Plan Builder, it must be backed by the runtime path from Slice 3.
- The page must not imply that upcoming steps have produced artifacts.

## Acceptance Criteria

- A user can paste source material without naming the orchestration first.
- A user can attach files before submit.
- The initial screen does not expose Expected Shape.
- The initial screen does not list instantiator/root-startup details as active or blocked work.
- The UI shows a clear current action immediately after submit.
- Unsupported runtime behavior remains visible and understandable without looking like background processing.
- Existing tests are updated so title-before-submit is no longer required unless an implementation decision explicitly keeps an internal title field.

## Root Decisions Before Delegation

The root orchestrator should decide:

- whether this slice should land before or after the first real runtime start path
- what the button should say in the interim if runtime is still unsupported
- whether the folder picker stays hidden, optional, or deferred until instantiation
