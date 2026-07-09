# Slice 6: End-To-End Usability Verification

## Goal

Verify the full Add Orchestration build flow as a user experience after the agent-backed pieces are implemented.

This slice checks attention flow, not only passing tests.

## Problem

The observed user problem was that the app did not make it clear what was happening. A user could submit a prompt and then land on a screen where "integration pending" looked ambiguous. It was unclear whether the agent was working, whether the user needed to do something, or whether the system was blocked.

The final implementation should be tested against that exact failure mode.

## Verification Scenario

Run the Add Orchestration flow with a substantial source prompt, such as the orchestration handoff material used earlier in this thread.

Verify these moments:

1. Add Orchestration entry.
2. Empty Plan Builder intake.
3. Source material pasted or attached.
4. Submit clicked.
5. Immediate feedback after submit.
6. Backend/runtime acknowledgement.
7. Waiting for first runtime event.
8. Runtime activity visible.
9. Plan Builder output complete.
10. Feedback re-prompt sent.
11. Updated planner response complete.
12. Confirm plan and start instantiation.
13. Instantiator processing visible.
14. Expected Shape hidden before instantiator output.
15. Expected Shape visible after instantiator output.
16. Approve and initiate orchestration.
17. Initiation artifacts visible.
18. Conversation windows visible if conversations are created.
19. Navigation to normal orchestration view.

If any step is unsupported in the current implementation, the UI must say so directly and must not look like hidden background work.

## Evidence To Capture

Capture:

- screenshots of each major state
- notes on whether the user knows what is happening now
- console/backend errors
- persisted records or event evidence where relevant
- storybook states for reusable components
- automated tests that cover critical state transitions

## UX Acceptance Checklist

- Every action has immediate feedback.
- Long-running work has a visible current action.
- The user can distinguish waiting, running, completed, failed, unsupported, and not started.
- Conversation output appears where the user submitted input.
- The user is not moved to a different context without explanation.
- Expected Shape appears only after instantiation evidence.
- Initiation artifacts are tracked from real records.
- The final navigation target is real.

## Technical Acceptance Checklist

- Unit tests cover state transitions and provenance rules.
- Component or Storybook states cover the reusable conversation view and conversation window.
- Integration tests cover unsupported runtime behavior.
- Runtime tests cover successful and failed Plan Builder start when the runtime path exists.
- Build, lint, format, and relevant Tauri/Rust checks pass.

## Scope

In scope:

- Hands-on usability pass.
- Test and story gaps discovered by that pass.
- Small corrective changes required to make state feedback clear.
- Documentation of remaining unsupported behavior.

Out of scope:

- Adding new product capabilities not already implemented by prior slices.
- Re-opening earlier architecture decisions unless verification finds a real blocker.

## Acceptance Criteria

- A review report is written with screenshots or clear evidence.
- The report explicitly answers whether a user can follow what is happening now.
- Any unsupported behavior is visible and understandable.
- No UI state claims runtime activity without evidence.
- The flow either completes into the normal orchestration view or stops at an honest unsupported state.

## Root Decisions Before Delegation

The root orchestrator should decide:

- whether the verification worker should run against Tauri desktop, browser dev mode, or both
- which source prompt should be used for the final scenario
- whether discovered issues are fixed inside this slice or returned as follow-up corrections
