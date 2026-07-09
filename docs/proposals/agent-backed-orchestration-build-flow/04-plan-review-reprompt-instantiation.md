# Slice 4: Plan Review, Re-Prompt, And Instantiation Gate

## Goal

Let the user review Plan Builder output, continue the same agent conversation with feedback, and explicitly start the instantiation step only when they approve the plan.

This slice turns Plan Builder completion into a real user decision gate.

## Problem

The current build screen shows a conversation and Expected Shape tab immediately after draft creation. Re-prompting adds a local context note rather than sending feedback to the same agent conversation. The app also shows later stage concepts before Plan Builder output exists.

The target flow is different:

- Plan Builder output is reviewed in the same conversation where the source material was submitted.
- The user can re-prompt the planner with feedback.
- The user can explicitly confirm the plan and start instantiation.
- Instantiation continues the same conversation with a fixed prompt.
- Expected Shape stays hidden until instantiation output exists.

## Proposed Change

After Plan Builder completes:

1. Keep the user in the same conversation view.
2. Show Plan Builder as no longer processing.
3. Show a user action such as `Confirm build plan and start instantiating`.
4. Keep the prompt box available for feedback.
5. When the user submits feedback, send it to the same conversation if the runtime supports continuation.
6. If continuation is unsupported, say so plainly and preserve the feedback locally.
7. When the user confirms the build plan, send a fixed instantiation prompt through the same conversation route.
8. Mark Plan Builder done only at the approval boundary, not merely because an assistant message exists.
9. Mark Instantiator processing only after the runtime start is accepted and/or runtime events arrive.

## Instantiation Prompt

The fixed prompt should instruct the agent to use the approved plan and invoke the instantiation behavior. It should not be invented blindly.

Before implementing this slice, inspect the local instantiator skill and current orchestration package expectations. The prompt should be specific enough to start the intended step but should not claim any output path, file shape, or thread id that the runtime has not provided.

## Expected Shape Gating

Expected Shape should become visible only after instantiator output exists.

Before instantiator output:

- no Expected Shape tab
- no generated package panel implying output slots are active
- no `orchestration-plan.json` as a real generated artifact
- no root startup prompt slots as real generated artifacts

After instantiator output:

- show generated or proposed structure with provenance
- distinguish files actually written from files proposed or expected
- show validation status if validation exists
- show missing files as missing, not pending

## Scope

In scope:

- Review/approval gate after Plan Builder output.
- Feedback prompt continuation through the reusable conversation view.
- Instantiator start action.
- Stage transitions for Plan Builder and Instantiator.
- Hiding Expected Shape until instantiator evidence exists.
- Tests for gating and re-prompt behavior.

Out of scope:

- Initiation/root startup artifact tracking.
- Full normal orchestration workspace integration.

## UX Requirements

- The user should understand that Plan Builder has produced a proposal, not that the whole orchestration has started.
- The primary next action should be an explicit approval action.
- Feedback should feel like continuing the same conversation, not adding local notes.
- Instantiator processing should be visible in the left stage outline.
- Expected Shape should feel earned by generated material, not pre-rendered.

## Acceptance Criteria

- Plan Builder output appears in the same conversation view.
- User feedback is submitted to the same runtime conversation when supported.
- Unsupported continuation is explicit and non-misleading.
- The confirm/instantiate action sends a fixed prompt through the runtime path.
- Plan Builder and Instantiator stage states reflect real events.
- Expected Shape is hidden before instantiator output.
- Expected Shape appears after instantiator evidence exists and labels provenance clearly.

## Root Decisions Before Delegation

The root orchestrator should decide:

- whether approval should mark Plan Builder completed before or after instantiator start succeeds
- what minimum instantiator output is required before Expected Shape appears
- how to handle runtime routes that can start new turns but cannot preserve full conversation context
