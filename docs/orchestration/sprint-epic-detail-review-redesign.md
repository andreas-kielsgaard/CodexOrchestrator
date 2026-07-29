# Sprint and Epic detail review redesign

Status: recorded product-review correction; no live orchestration support added.

## Decisions

- `problems` owns explicit many-to-many links from Sprint Planner problems to Planner Activities,
  Work Units, and gates. No link is inferred from transcript prose or layout.
- `epicPlannerObjectives` is a sourced per-Sprint list. The global Epic goal is not relabeled as a
  Sprint objective.
- `workUnitLifecycle` identifies the exact Agent Session and invocation for each chronological
  entry. A lifecycle Session must have either the exact Work Unit execution reference or the exact
  owning Planner Activity reference whose typed membership contains that Work Unit scope.
- the recorded WU-RD1 lifecycle includes its Sprint Planner action, worker turns, and Work Unit
  handler actions. The same handler performs review, reprompt, merge, and completion, so no separate
  Reviewer Session is recorded.
- lifecycle invocation identifiers remain recorded-only because no authoritative invocation
  registry exists. This metadata is not a runtime lifecycle or persistence seam.
- `ResizableSplitSurface` is the shared pointer- and keyboard-draggable boundary for vertical
  Flow/Session and horizontal Agent Session layouts. Its separator owns dynamic value semantics;
  maximize remains a separate control.
- the focusable Sprint context rail scrolls independently on desktop and compact layouts.
- Sprint Documents open the normalized `FileReviewSource` boundary. Product detail does not select
  repositories, worktrees, paths, or source adapters.
- not-started Sprints expose their typed proposed Plan. Historical revisions without a recorded
  workflow show an unavailable state.
- current Plan and Work Unit state is a distinct sourced presentation element. Product-facing copy
  omits fixture notices, Direction, raw attempt labels, Responsibility Accepted, and fixed-scope
  implementation language.

## Recorded review composition

`Sprint and Epic Detail Review` is an in-progress development example with two Plan revisions,
parallel groups, cross-group dependencies, mixed Work Unit states, and later divergent work. Its
concerns intentionally distribute Work Units rather than assigning every Work Unit to every
concern.

The example and Agent Session turns are recorded-only. They do not prove durable planning,
execution, review, merge, continuation, or restart behavior.

## Review evidence

Automated coverage exercises:

- Epic Planner objectives, Sprint Planner problems, mixed-state horizontal flow, highlighting, and
  repeated focus order;
- proposed not-started Plans, historical revisions, concerns, Documents, and normalized file review;
- nine WU-RD1 lifecycle entries, including exact Sprint Planner and same-handler review turn focus;
- pointer/keyboard resizers, compact axis changes, independent context scrolling, and route focus
  recovery;
- Agent Sessions -> Work Unit -> Agent Sessions identity round trips.

Desktop and narrow browser evidence is refreshed with each review checkpoint. It is responsive
development-browser evidence for the recorded composition, not live runtime evidence.

## Deferred seams

- runtime producers may later supply problem links and lifecycle entries through a durable
  application read boundary;
- a durable invocation registry could replace recorded invocation navigation metadata;
- Agent-generation quality and concern-to-Work-Unit distribution remain later test-platform work.
