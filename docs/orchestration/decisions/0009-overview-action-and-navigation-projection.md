# Overview action and navigation projection

Status: implemented as a recorded/product-boundary correction; not merged or accepted.

## Decision

The Orchestration overview consumes four independently sourced application projections:

- lifecycle state: `running`, `paused`, `blocked`, or `completed`
- movement: zero or more typed `processing` and `reviewing` items
- ready work: specific application-owned actions
- human input: either one exact waiting action or no action

`ready_to_continue` is removed. Ready work may be present while movement is active. An available
empty movement list renders as non-interactive `No work in motion`; unavailable movement renders its
source limit and is also non-interactive.

Overview navigation uses `ProductOverviewNavigationTargetV1` for Epic, Sprint, Sprint Planner
activity, and Work Unit locations. The row alone uses the Epic-root target. Movement, ready-work,
and human-input items use the typed non-Epic subset, so an exact action cannot degrade to opening the
Epic root. Composition validates the full ownership path before the feature receives it. The feature
never derives readiness or a destination from transcript prose, titles, routes, or mounted view
state.

The whole Epic row opens the Epic through one button plus row click delegation. Tooltip, movement,
ready-work, and human-input controls are sibling controls, so no interactive element is nested in
another.

## Boundary and deferrals

This is not a general human-in-the-loop model. A later model must own action lifecycle, ordering,
resolution, persistence, authorization, and multiple concurrent decisions. This slice only projects
the one application-owned action currently waiting for overview navigation.

The target union intentionally remains separate from the active Sprint-detail and Agent Sessions
redesigns. Later consolidation can share target shapes or add an Agent Session target through the app
router without making this overview depend on either unfinished worktree.

The native initiated-Epic query still reports movement, lifecycle, ready work, and human input as
unavailable until those facts have product sources. The development fixture records an initiated Epic
with zero movement and one specific next action. Focused UI evidence separately covers simultaneous
movement, ready work, and a waiting human-input action; it is not production runtime proof.

## Recorded browser evidence

The development composition was inspected in the in-app browser on `2026-07-29`:

- At `1280 x 800`, document width equaled viewport width and the focused description tooltip was
  visible outside the table row without an internal scrollbar.
- At `390 x 844`, document width remained `390`, the table/card stayed within `361` pixels, and the
  tooltip bounds were `37.6` to `367.6` pixels.
- Activating the recorded ready-work control opened the exact
  `Planner and Work Unit Interaction Discovery` Sprint detail and selected revision
  `sprint-planner-work-unit-r1`.

Screenshots were reviewed in-session and are not committed artifacts. The recorded fixture has zero
movement, so movement-popover behavior is covered by focused component tests rather than claimed as
browser/runtime proof.

## Validation

- Focused overview/application suites: 5 files, 84 tests passed.
- Full frontend suite: 88 files, 607 tests passed.
- `npm run build`, `npm run lint`, changed-file Prettier check, and `git diff --check` passed.
- Repository-wide `npm run format:check` remains red on 27 unchanged baseline files under `.agents`,
  `offline-review`, and existing Rust fixture paths.
