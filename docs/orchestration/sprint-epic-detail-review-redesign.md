# Sprint and Epic detail review redesign

Status: recorded product-review correction; no live orchestration support added.

## Decisions

- `sprintRunnerConcerns` owns explicit many-to-many links from Sprint Runner concerns to Work Slice
  planning points, Work Units, and gates. No link is inferred from transcript prose or layout.
- `epicRunnerObjectives` is a sourced per-Sprint list. The global Epic goal is not relabeled as a
  Sprint objective.
- `workUnitLifecycle` identifies the exact Agent Session and invocation for each chronological
  entry. A lifecycle Session must have either the exact Work Unit execution reference or the exact
  owning planning-point reference whose typed membership contains that Work Unit scope.
- the recorded WU-RD1 lifecycle includes its Work Slice Planner action, Work Unit Implementer turns,
  and Work Unit Handler actions. The same Handler performs review, reprompt, integration, and
  completion, so no separate
  Reviewer Session is recorded.
- recorded WU-ECS2E retains both attempt outcomes. Its Work Unit Implementer owns the implementation
  returns; its one Work Unit Handler owns review, reprompt, and acceptance. Its Work Slice Planner
  remains a separate Session and lifecycle actor; no Reviewer role or Session exists.
- lifecycle invocation identifiers remain recorded-only because no authoritative invocation
  registry exists. This metadata is not a runtime lifecycle or persistence seam.
- `ResizableSplitSurface` is the shared pointer- and keyboard-draggable boundary for vertical
  Flow/Session and horizontal Agent Session layouts. Its separator owns dynamic value semantics;
  maximize remains a separate control.
- the focusable Sprint context rail scrolls independently on desktop and compact layouts.
- Sprint Documents open the normalized `FileReviewSource` boundary. Product detail does not select
  repositories, worktrees, paths, or source adapters.
- not-started Sprints expose a typed low-resolution concern forecast, not concrete Work Units.
  Started Sprints require a recorded repository reevaluation before their higher-resolution Plan,
  Work Units, dependencies, lanes, or Work Slice planning-point views are available.
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

- Epic Runner objectives, Sprint Runner concerns, mixed-state horizontal flow, highlighting, and
  repeated focus order;
- concerns-only pre-start forecasts, started Plans, historical revisions, Documents, and normalized file review;
- nine WU-RD1 lifecycle entries, including exact Work Slice Planner and same-Handler review turn focus;
- the WU-ECS2E Agent Sessions -> Work Unit -> Agent Sessions round trip, with one handler identity,
  separate Planner, Handler, and Implementer conversations, and no Reviewer surface;
- pointer/keyboard resizers, compact axis changes, independent context scrolling, and route focus
  recovery;
- Agent Sessions -> Work Unit -> Agent Sessions identity round trips.

Desktop and narrow browser evidence is refreshed with each review checkpoint. It is responsive
development-browser evidence for the recorded composition, not live runtime evidence.

## Deferred seams

- runtime producers may later supply concern links and lifecycle entries through a durable
  application read boundary;
- production start and repository-reevaluation producers remain unavailable; the current examples
  are recorded states only;
- a durable invocation registry could replace recorded invocation navigation metadata;
- Agent-generation quality and concern-to-Work-Unit distribution remain later test-platform work.
