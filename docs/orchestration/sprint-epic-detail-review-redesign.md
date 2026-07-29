# Sprint and Epic detail review redesign

Status: recorded product-review correction; no live orchestration support added.

## Decisions

- `ProductSprintWorkspacePresentationMetadataV1.problems` owns explicit many-to-many links from
  Sprint Planner problems to Planner Activities, Work Units, and gates. The product does not infer
  these links from transcript prose or layout.
- `epicPlannerObjectives` is an explicit sourced per-Sprint list authored by the Epic Planner. The
  composer validates each objective identity, source, and Sprint owner; the global Epic goal is not
  relabeled or projected as a Sprint objective.
- `workUnitLifecycle` is recorded navigation metadata. It identifies the responsible Agent Session
  and invocation for each chronological entry. The composer requires a same-Sprint Work Unit scope,
  a Work Unit execution relationship for the Agent Session, and unique sequence numbers per Work
  Unit. Invocation identifiers remain recorded-only because no authoritative invocation registry
  exists; this is not a runtime lifecycle or persistence seam.
- `ResizableSplitSurface` is the shared two-pane boundary for vertical Flow/Session and horizontal
  Agent Session layouts. Pointer and keyboard resizing keep both panes mounted. The separator owns
  dynamic range/value semantics and stable pane controls; the maximize action remains a separate
  accessible control.
- The focusable Sprint context rail scrolls independently on desktop and compact layouts, so long
  objective/problem lists remain reachable without document-level overflow.
- Sprint Documents open the accepted normalized `FileReviewSource` boundary. Application-owned
  sources default to complete Document content and expose a Sprint-start comparison. The product
  detail surface does not select repositories, worktrees, paths, or source adapters.
- Historical plan revisions without a recorded workflow continue to show an unavailable state.

## Recorded review composition

`Sprint and Epic Detail Review` is an in-progress development fixture with two plan revisions,
parallel groups, cross-group dependencies, mixed Work Unit states, and later divergent work. Its
concerns intentionally distribute Work Units rather than assigning every Work Unit to every
concern.

The fixture and Agent Session turns are recorded-only. They do not prove durable planning,
execution, review, merge, continuation, or restart behavior.

## Review evidence

Run `npm run dev -- --host 127.0.0.1 --port 4179 --strictPort`, then open
`http://127.0.0.1:4179/?file-diff-viewer`. The recorded review route was exercised at 1440 x 900
and 640 x 900:

- the Sprint context retained four Epic Planner Sprint objectives beside three separate Sprint
  Planner problems;
- at 640 x 900 the 159 px context viewport exposed 380 px of independently scrollable content,
  reached the final problem, and kept document width equal to the 640 px viewport;
- the mixed-state RD-R2 graph showed parallel dependencies and WU-RD6 as later divergence;
- repeated problem activation focused processing, completed, then planned graph elements;
- WU-RD1 showed eight lifecycle entries and focused the recorded reprompt invocation;
- the Flow/Session separator updated its value for Home, Arrow, End, and maximize while its
  maximize button remained outside the separator role and controlled the Flow pane;
- the horizontal Agent Session split stacked vertically with a horizontal separator at the
  compact breakpoint, updated its value bounds after the axis change, and avoided document-level
  horizontal overflow;
- the review Sprint Document opened as complete Markdown and compared against its recorded
  Sprint-start state without a source selector.

The fresh correction run recorded no browser console warnings or errors.

This is responsive development-browser evidence for the recorded composition, not live runtime
evidence.

## Deferred seams

- Runtime producers may later supply problem links and lifecycle entries through a durable
  application read boundary.
- Agent-generation prompt quality, including concern-to-Work-Unit distribution, remains a later
  test-platform evaluation. This correction changes only misleading recorded fixtures.
