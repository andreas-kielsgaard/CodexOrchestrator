# Orchestration Orientation Discovery final handoff

Status: accepted direction converged for G3 review without merge. G2 user evaluation passed before
WU-OD6. This document replaces the disposable fixture, A/B comparison, room, integration, minimal
section, and plan-refinement notes.

## Accepted direction

- The actual app opens **Orchestrations** by default and retains **Agent Sessions** as a peer
  destination.
- The overview is a three-column table: Orchestration, Current movement, and State.
- Detail is a fixed, four-edge-contained workspace with a compact context rail, an ordered colored
  Epoch plan, and the linked Orchestrator Agent Session.
- The collapsed session uses the reusable `AgentSessionExcerpt` with a structured transcript range;
  expansion uses the recorded `ConversationViewport`. No orchestration-specific latest-turn string
  renderer is retained.
- Auto-flow is a compact local projection of automatic continuation intent. It does not execute,
  persist, or prove eligibility.
- Completed and current Epoch items open a restrained accessible placeholder dialog. Future Epochs
  are inert. Full Epoch detail navigation and design are deferred to Epoch Control Surface
  Discovery.

The structured movement, state, ordered-plan, blocker, Agent Session reference, and continuation
shapes in `src/features/orchestrations` are provisional presentation contracts. They are disposable
and must not be treated as durable orchestration domain types, persistence records, a routing
contract, or a transition engine.

## Evaluation evidence and decisions

Early development-only work compared a dense movement-path overview with a focused re-entry
overview, plus beside-conversation and framing-conversation detail compositions. G1 used recorded
state separation and desktop/narrow inspection to identify the useful pieces. G2 accepted the
actual app composition summarized above, including conversation primacy, a compact context rail,
fixed workspace containment, the ordered Epoch plan, the reusable Agent Session excerpt, compact
Auto-flow, and the restrained started-Epoch placeholder.

The A/B selector, harness chrome, old current-movement and attention summary cards, standalone
orientation HTML entries, and their screenshot set were discovery instruments rather than product
surface. WU-OD6 removed them after import reachability showed that only one recorded Agent Session
projection was still needed. That minimal projection now lives in
`src/dev/orchestrationSection/disposableRecordedOrchestrationView.ts` and is explicitly labelled as
a development-only, non-executing presentation fixture.

## Explicit deferrals

- Defaults, persistence, permissions, confirmation behavior, and final runtime placement for
  automatic continuation remain unresolved.
- No durable orchestration schema, orchestration language, route model, or transition engine exists.
- Production persistence, production runtime integration, live or paid Codex execution, automatic
  continuation execution, and completion delivery are forbidden for this discovery result and
  deferred.
- The placeholder Epoch dialog is not an accepted Epoch control surface. Navigation, supervision,
  editing, and control behavior belong to the next discovery Epoch.

## File disposition

| Disposition | Files                                                                                                                                | Reason                                                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Retained    | `src/features/orchestrations/` and tests                                                                                             | Accepted app-facing overview/detail presentation and interaction coverage.                                                       |
| Retained    | `src/features/agentSessions/` additions and tests                                                                                    | Reusable workspace, excerpt, recorded conversation viewport, transcript anchors/ranges/projectors, and controller repairs.       |
| Retained    | `src/dev/agentSessions/`, `agent-session-harness.html`, Tauri active composition/storage, and Preparation documents                  | Accepted neutral foundation and deterministic harness remain source-valued.                                                      |
| Relocated   | Required recorded transcript and orchestration projection into `src/dev/orchestrationSection/disposableRecordedOrchestrationView.ts` | Keeps the actual app deterministic without retaining the broad orientation fixture domain.                                       |
| Removed     | `orientation-discovery-harness.html`, `orientation-fixture-harness.html`, and their Vite inputs                                      | Superseded build entrypoints; no accepted consumer remained.                                                                     |
| Removed     | `src/dev/orientationDiscovery/`, `orientationFixtures/`, `orientationOverview/`, and `orientationRoom/`                              | Unreachable A/B, fixture, and room scaffolding superseded by the accepted app surface.                                           |
| Removed     | Superseded orientation comparison/evaluation notes and `docs/agent-session/evidence/orientation-discovery/`                          | Their accepted decisions and rejected-hypothesis history are consolidated here; screenshots were disposable evaluation evidence. |

## Validation matrix

| Concern                                                     | Evidence                                                                                                                           | Result                                                                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency reachability                                     | Repository-wide import and entrypoint search before deletion                                                                       | Passed: only the recorded fixture dependency required relocation.                                                                                     |
| Orchestrations and reusable Agent Session regressions       | Focused Vitest run across OrchestrationSection, app navigation, excerpt, workspace, viewport, transcript projector, and controller | Passed: 7 files, 40 tests.                                                                                                                            |
| Frontend formatting, lint, full tests, and production build | `npm run format:check`, `npm run lint`, `npm test`, `npm run build`                                                                | Passed: 61 files and 363 tests; build emits only the normal app and neutral Agent Session harness HTML entries.                                       |
| Rust non-live behavior                                      | `cargo test` with intentional live/CLI tests left ignored, `cargo check`, `cargo fmt --check` from `src-tauri`                     | Passed: 87 tests; two intentional ignores; check and format pass with existing dead-code warnings only.                                               |
| Diff hygiene                                                | `git diff --check`, final status, artifact/build-entry search, and process audit                                                   | Passed: no whitespace errors, staged changes, obsolete orientation entries/assets, workspace dev server, native app, or listeners on ports 1420/4173. |

## Repository state and proposed commit partition

WU-OD6 remains an unstaged, uncommitted dirty-worktree result on `main`. It performed no reset,
checkout, branch creation, staging, commit, merge, push, live/paid Codex invocation, production
persistence, schema, or transition-engine work.

After explicit user authorization, a practical partition is:

1. **Preparation foundation**: Tauri active composition and disposable Agent Session storage;
   Agent Session workspace, transcript projection/ranges, excerpt, viewport, controller repairs;
   neutral harness; associated tests and source-valued Preparation documents.
2. **Orientation discovery**: app navigation/default surface; `src/features/orchestrations`;
   disposable recorded orchestration presentation fixture; Vite entry cleanup; this final handoff;
   deletion of superseded discovery scaffolding and evidence.

Because both partitions currently coexist in one accepted dirty checkout, use file-level and
hunk-level review before staging. Do not split overlapping Agent Session files mechanically.

## Recommended next-Epoch input

Begin **Epoch Control Surface Discovery** from the accepted detail workspace, not from the removed
harness variants. Determine the information and interaction contract for inspecting and
supervising one started Epoch, including navigation and return behavior, without promoting the
current presentation types into durable domain contracts. Keep automatic-continuation defaults,
permissions, confirmation, persistence, and runtime execution outside that discovery unless the
user separately authorizes a later implementation Epoch.

Use the same app-mounted components for recorded evaluation and eventual product behavior. During
discovery, recorded adapters may supply presentation state and non-executing controls. A later
integration Epoch must replace those adapters with application-owned data connectors, Agent Session
controllers, and continuation controllers without replacing the accepted visual components.

Future Epochs must not depend on the quarantined task/run commands or add behavior to
`src-tauri/src/lib.rs`. When an orchestration requirement resembles a legacy capability, define its
current requirement explicitly and implement it through a focused domain/application port. The
current trajectory and deferred review items are recorded in
`docs/orchestration/future-sprint-trajectory.md` and
`docs/orchestration/post-orchestration-review-notes.md`.
