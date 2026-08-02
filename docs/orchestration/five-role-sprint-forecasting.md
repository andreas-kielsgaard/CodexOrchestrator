# Five-role and Sprint forecasting contract

Status: implemented application/read-model contract and recorded review composition. This record
does not claim production start orchestration, repository reevaluation, merge, release, or user
acceptance.

## Implemented

- Agent Session references accept exactly Epic Runner, Sprint Runner, Work Slice Planner, Work Unit
  Handler, and Work Unit Implementer. Target compatibility is decoded at the application boundary.
- one Work Slice planning point can reference at most one Work Slice Planner Session. The planning
  point is temporal evidence, not a recurring Planner identity.
- a not-started Sprint can expose only `pre_start_forecast`; its one application-managed Sprint
  plan shows proposed objectives, sourced concerns, and forecast task breakdown while withholding
  Work Units, dependencies, lanes, tabs, Sessions, and planning-point detail.
- `started_plan` requires a current planning point belonging to the Sprint, a non-empty recorded
  branch/repository assessment, and a valid reevaluation time. Only then are the horizontal Sprint
  Runner map and Work Slice planning-point/Work Unit views available.
- each recorded Work Unit lane names its Handler and Implementer. WU-ECS2E and WU-RD1 keep Planner,
  Handler, and Implementer Sessions distinct; review, correction, and integration remain Handler
  actions and no Reviewer Session is created.
- Work Slice planning-point detail requires an actually instantiated Work Slice Planner. Its causal
  timeline comes from typed selected-revision membership, role reports, Work Units, executions, and
  Agent Session references. It retains every scoped Work Unit and explicitly marks missing Handler
  or Implementer relationships unavailable.
- role-specific conformance records admit Sprint plan refinement, Epic oversight, Planner analysis
  and dependencies, Handler/Worker activity, and lifecycle transitions only through their matching
  typed five-role Agent Session bindings. The decoder rejects transcript, message, prompt, Harness,
  and route inference fields.
- dependency reports distinguish functional output, shared-resource/workspace exclusion, and
  merge/join dependencies. Merge/join records additionally distinguish a merged result from
  independently completed prerequisites. An independent-prerequisite record names at least two
  unique typed inputs and one target; the timeline draws their separate completion legs into a
  hollow gate. A merged-result record remains one solid output edge with a distinct marker. Neither
  grouping nor geometry is inferred from labels or node positions.
- Agent Sessions and product routes reuse the same typed Session IDs and application-owned
  destinations. Titles, transcripts, route state, Harness text, and visual position are not
  relationship authority.

## Deferred

- no production command currently performs Sprint start or repository reevaluation and writes this
  planning state; production reads remain unavailable without such records.
- recurring Work Unit types, recurring Work Slice Planner semantics, specialized Implementer skill
  profiles, and further role splits are not modeled.
- production producers for the role-report contract are not connected. Recorded reports make the
  development composition reviewable but do not prove provider launch, execution, persistence,
  review, correction, integration, or restart recovery.
