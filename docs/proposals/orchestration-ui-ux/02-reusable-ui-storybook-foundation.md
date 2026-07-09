# Slice 2: Reusable UI and Storybook Foundation

## Goal

Extract reusable UI components and add Storybook so orchestration states can be reviewed in isolation without relying on the full Tauri app or simulated runtime progress.

This slice should make the UI easier to reason about before additional orchestration behavior is added.

## Problem

The current app has reusable visual ideas, but they are mostly expressed as CSS classes and local functions inside `src/app/App.tsx`. Buttons, icon buttons, status pills, tabs, panels, conversation windows, file lists, and stage boards are repeated directly in flow code.

This makes UI changes harder to verify because the only practical path is rendering the whole app. It also makes truthful state design harder, because there is no single place to enforce how pending, running, mock, unsupported, failed, and completed states should look.

Storybook is also not currently configured. That leaves no isolated workspace for reviewing loading, waiting, failure, long-content, and incomplete-integration states.

## Proposed Change

Create a small reusable UI layer and Storybook setup.

Recommended components:

- `Button`
- `IconButton`
- `StatusPill`
- `Tabs`
- `Panel`
- `EmptyState`
- `FieldGroup`
- `ConversationThread`
- `ConversationComposer`
- `FileList`
- `StageList`
- `ActivityTimeline`
- `CurrentAction`

The first extraction should be conservative. Avoid building a large design system before the product has settled. Extract the pieces that are repeated now and needed by orchestration flow states.

## Storybook Requirements

Storybook should run without Tauri.

Stories should use explicit mock fixtures and labels:

- Draft orchestration
- Ready to submit prompt
- Submitting prompt
- Waiting for backend acknowledgement
- Integration pending
- Runtime running
- Runtime failed
- Completed from real-looking event fixture
- Mock preview
- Long prompt and long file names
- Empty uploaded files
- Many conversation messages

Mock fixtures should not be written as though they are product truth. In stories, use names like `mockDraftOrchestration`, `mockRuntimeRunningEvent`, and `mockIntegrationPendingState`.

## Scope

In scope:

- Add Storybook scripts and configuration.
- Extract a small UI component layer.
- Add stories for the reusable components.
- Add stories for Add Orchestration states after the truthful state model exists.
- Keep stories independent from Tauri commands.

Out of scope:

- Full redesign of the entire app.
- Adding Radix or another component library unless a specific component need justifies it.
- Real runtime integration.
- Pixel-perfect polishing of every current screen.

## UX Requirements

Reusable components should encode attention-flow behavior:

- Buttons show disabled, busy, and error-adjacent states consistently.
- Status pills distinguish local, pending, running, failed, completed, unsupported, and mock states.
- Stage lists show "not started," "ready," "waiting," and "running" honestly.
- Conversation components show whether a message is user input, local pending, backend accepted, runtime output, or mock.
- Current-action displays answer "what is happening now" without inventing details.

## Acceptance Criteria

- `npm run storybook` or equivalent starts Storybook.
- At least the core components have stories.
- Orchestration stories cover draft, ready, pending, unsupported, running, failed, and completed states.
- No Storybook fixture is named or labeled in a way that implies it is real runtime data.
- Existing app UI can begin consuming extracted components without changing behavior.

## Suggested Implementation Notes

Start with low-risk components:

1. `Button` and `IconButton`
2. `StatusPill`
3. `Tabs`
4. `Panel`
5. `ConversationThread`
6. `StageList`
7. `CurrentAction`

After the components exist, migrate orchestration flow UI first. Task dashboard components can migrate later when there is a reason.

Keep CSS migration incremental. The first pass can wrap existing classes, then gradually move toward component-scoped class names and typed variants.
