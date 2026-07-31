# Five-role and Sprint forecasting contract

Status: implemented application/read-model contract and recorded review composition. This record
does not claim production start orchestration, repository reevaluation, merge, release, or user
acceptance.

## Implemented

- Agent Session references accept exactly Epic Runner, Sprint Runner, Work Slice Planner, Work Unit
  Handler, and Work Unit Implementer. Target compatibility is decoded at the application boundary.
- one Work Slice planning point can reference at most one Work Slice Planner Session. The planning
  point is temporal evidence, not a recurring Planner identity.
- a not-started Sprint can expose only `pre_start_forecast`; the product surface shows sourced
  concerns and withholds Work Units, dependencies, lanes, tabs, and planning-point detail.
- `started_plan` requires a current planning point belonging to the Sprint, a non-empty recorded
  branch/repository assessment, and a valid reevaluation time. Only then are the horizontal Sprint
  Runner map and Work Slice planning-point/Work Unit views available.
- each recorded Work Unit lane names its Handler and Implementer. WU-ECS2E and WU-RD1 keep Planner,
  Handler, and Implementer Sessions distinct; review, correction, and integration remain Handler
  actions and no Reviewer Session is created.
- Agent Sessions and product routes reuse the same typed Session IDs and application-owned
  destinations. Titles, transcripts, route state, Harness text, and visual position are not
  relationship authority.

## Deferred

- no production command currently performs Sprint start or repository reevaluation and writes this
  planning state; production reads remain unavailable without such records.
- recurring Work Unit types, recurring Work Slice Planner semantics, specialized Implementer skill
  profiles, and further role splits are not modeled.
- the workflow map remains recorded presentation data and does not prove provider launch,
  execution, persistence, review, correction, integration, or restart recovery.
