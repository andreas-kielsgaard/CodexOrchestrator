# Frontend Architecture Contracts

## Purpose And Scope

This note tracks the ongoing React/TypeScript `src` migration by technical concern:
app shell, features, views, UI controllers, view-model/presenter helpers, reusable
capabilities, application use cases/contracts, domain rules, and infrastructure adapters.

CSS is intentionally out of scope for the migration waves. Generic cleanup work is also out
of scope unless it is necessary to preserve a boundary while moving a slice.

## Dependency Direction

Target direction:

```text
main -> app + infrastructure
app -> features + application contracts + app shell controllers/views/view models
features -> application contracts + feature controllers/views/view models
views -> rendering-only shared primitives
capabilities -> application contracts and DTOs
application -> domain + application ports
domain -> domain only
infrastructure -> application/domain ports and contracts
```

Avoid these directions:

```text
domain -> application
domain -> infrastructure
application -> app/features
app/features -> concrete infrastructure adapters
shared views -> feature/app controllers
```

`src/main.tsx` is allowed to compose concrete infrastructure clients and pass them to
`AppRoot`. React feature code should consume injected contracts, not instantiate Tauri,
SQLite, Git, Codex, or filesystem adapters.

## Current Migration State

Completed frontend migration slices:

- `src/app/App.tsx` is now a compatibility export for `AppRoot`.
- `src/app/AppRoot.tsx` owns top-level feature composition and dependency injection.
- `src/features/openTasks/OpenTasksPage.tsx` is now a thin feature entry point that receives
  Open Tasks capabilities, calls `useOpenTasksFeatureController`, and renders `OpenTasksScreen`.
- `src/features/openTasks/controllers/useOpenTasksFeatureController.ts` owns Open Tasks workflow
  coordination across leaf controllers, including aggregate busy/error state, cross-controller
  task detail coordination, and view DTO projection.
- `src/features/openTasks/views` owns Open Tasks rendering components:
  - Open Tasks screen composition
  - repo setup form
  - task composer form
  - task edit/run forms
  - task run detail panel
- `src/features/openTasks/controllers` owns Open Tasks UI flow state:
  - dashboard loading and dashboard writes
  - repo onboarding and discovery
  - task composition
  - task edit/archive/state changes
  - task run launching feedback
  - selected task detail state
- `src/app/controllers` now only holds app-shell or cross-feature controllers:
  - backend maintenance
  - runtime health/stale notice polling
- `src/app/views` now only holds app-shell rendering such as startup, sidebar, and runtime
  stale notices.
- `src/views` now holds reusable rendering primitives that are not owned by a single
  feature workflow. It currently contains the shared error notice.
- `src/features/openTasks/views` now also owns Open Tasks feature composition:
  - Open Tasks header
  - task review layout
  - task group/card composition
- `src/features/openTasks/viewModels` owns Open Tasks-specific labels, form shaping,
  task review/card DTOs, composer option DTOs, repo discovery option DTOs, task detail panel
  DTOs, artifact/validation/timeline shaping, and run-result summaries.
- `src/app/viewModels` now only holds app-shell or feature-agnostic helpers such as generic
  formatting and runtime stale-status formatting.
- `src/capabilities` now defines UI-facing capability contracts for Open Tasks dashboard
  operations, repo onboarding, task run launch/detail, runtime health, and backend maintenance.
  App/feature controllers depend on these contracts rather than broad application clients.
- `src/application/commands` now owns state-changing application use cases and command-facing
  contracts, with root-level compatibility re-exports left for existing callers.
- `src/application/queries` now owns read-only application query contracts for runtime health
  and task run detail, with root-level compatibility re-exports left for existing callers.
- `src/application/ports/gitRepoScanner.ts` now owns the Git repo scanner contract and
  scan-to-domain-facts mapper. Application commands no longer import Git infrastructure modules
  directly; Git infrastructure implements this port.
- `src-tauri/src` has been merged from the separate Rust refactor and is now split into
  Rust modules instead of one backend monolith.

This is materially closer to the stated objective: UI components now have directly serving
application-logic controllers, and those controllers consume injected technical contracts rather
than implementing persistence/runtime details themselves.

## Current Open Contracts

`AppRoot` currently receives these frontend-facing contracts:

- `TaskDashboardClient`: load dashboard data and mutate/register Open Tasks data.
- `TaskRunDetailClient`: load task run detail snapshots.
- `RuntimeCommandClient`: start Codex task runs.
- `RuntimeStatusClient`: poll runtime availability/staleness.
- `BackendMaintenanceClient`: check/reopen the Rust backend.

These concrete clients remain broad at the infrastructure/application implementation boundary,
but React controllers consume the narrower capability contracts in `src/capabilities`.

## Known Boundary Gaps

These are architectural gaps, not cleanup chores:

- Open Tasks still imports generic formatting helpers from `src/app/viewModels`. That is
  acceptable for now, but a future shared formatting home may be useful if more features need
  the same helpers.
- Root-level `src/application/*.ts` compatibility re-exports remain for existing imports.
  These should be removed only after consumers migrate to canonical command/query paths.
- `src/application/ports` and `presenters` are still mostly placeholders.
- `OpenTaskReviewLayout`, `TaskComposerForm`, `RepoSetupForm`, `TaskRunDetailPanel`, and
  `RuntimeStaleNotice` now consume UI-shaped DTOs and primitive callbacks instead of whole
  controllers or capability snapshots.
- Some runtime/app-shell state is still passed through `OpenTasksPage` because it is the only
  feature. That should be reconsidered when another feature or app shell composition exists.
- `GitRepoScanResult` is still shaped like parsed Git facts, although it is now an application
  port contract. A later refinement could make the port return domain-ready repo scan facts and
  keep raw parser output entirely private to infrastructure.

## Revised Remaining Waves

### Wave A: Boundary Tightening And Redundancy Check

Independent slices:

- Review `useOpenTasksFeatureController` for any coordination that should remain centralized
  versus any leaf-controller state/action surfaces that can be narrower.
- Review feature presenters for duplicate path/date/status formatting helpers before moving
  generic display helpers out of `src/app/viewModels`.
- Decide whether compatibility re-exports in root-level `src/application/*.ts` are still needed
  after all current imports have canonical command/query paths.
- Revisit `GitRepoScanResult` and decide whether the application port should return
  domain-ready scan facts rather than parser-shaped Git facts.

Parallelism:

- Feature-controller surface review can run independently from application compatibility export
  review.
- Git scanner port refinement can run independently from UI redundancy checks.

## Deferred But Not Forgotten

The redundancy pass remains intentionally deferred until the ownership moves settle. Specific
items to check later:

- duplicate disabled-state helpers after view-model moves
- broad clients that remain only as pass-through wrappers after capability extraction
- controllers exposing state/actions no view consumes
- compatibility exports that are no longer needed
- root-level application compatibility re-exports after callers use canonical paths
- whether `AppRoot` should remain a tiny composition root or gain routing/shell composition as
  additional features appear
