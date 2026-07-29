# Sprint and Epic detail review redesign

Status: recorded product-review correction; no live orchestration support added.

## Decisions

- `ProductSprintWorkspacePresentationMetadataV1.problems` owns explicit many-to-many links from
  Sprint Planner problems to Planner Activities, Work Units, and gates. The product does not infer
  these links from transcript prose or layout.
- `workUnitLifecycle` is recorded navigation metadata. It identifies the responsible Agent Session
  and invocation for each chronological entry; it is not a runtime lifecycle or persistence seam.
- `ResizableSplitSurface` is the shared two-pane boundary for vertical Flow/Session and horizontal
  Agent Session layouts. Pointer and keyboard resizing keep both panes mounted.
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

- the Sprint context retained the Epic objective beside three Sprint problems;
- the mixed-state RD-R2 graph showed parallel dependencies and WU-RD6 as later divergence;
- repeated problem activation focused processing, completed, then planned graph elements;
- WU-RD1 showed eight lifecycle entries and focused the recorded reprompt invocation;
- the horizontal Agent Session split stacked vertically with a horizontal separator at the
  compact breakpoint, without document-level horizontal overflow;
- the review Sprint Document opened as complete Markdown and compared against its recorded
  Sprint-start state without a source selector.

This is responsive development-browser evidence for the recorded composition, not live runtime
evidence.

## Deferred seams

- Runtime producers may later supply problem links and lifecycle entries through a durable
  application read boundary.
- Agent-generation prompt quality, including concern-to-Work-Unit distribution, remains a later
  test-platform evaluation. This correction changes only misleading recorded fixtures.
