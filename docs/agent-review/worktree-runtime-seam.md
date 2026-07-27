# Worktree review instance seam

## Status

This branch defines only the review-side prospective contract. It does not compose or copy the
worktree runtime. The read-only comparison used the independently accepted
`codex/explore-worktree-runtime` checkpoint at `a426a69041dc3ee6fbb2047e1ae356701f3726eb` on
2026-07-27.

The contract in `src/application/agentReview/worktreeRuntime.ts` mirrors the accepted semantic
facade. Every source is application-resolved; driver protocols remain inside development review
adapters.

## Semantic flow

1. The application resolves the current agent worktree to an opaque `TestSourceRef` and requests an
   isolated test instance with that reference and a short purpose.
2. The runtime returns an opaque instance handle and semantic status.
3. The caller invokes `build`, `test`, `start`, `status`, `stop`, or `recover` separately. Build and
   test return pass/fail, an optional failed step, and status. Lifecycle calls return phase, health,
   and staleness.
4. A development review adapter uses the handle through a future application-owned attachment and
   evidence port. That adapter, not the orchestration domain, resolves endpoints, windows, driver
   commands, and filesystem evidence locations.
5. Evidence references remain opaque across the neutral boundary. A separate judgement consumes
   evidence bundle IDs and does not gain runtime or driver authority.

## Request and result boundary

| Concern   | Exposed semantic value                                               | Hidden owner detail                                       |
| --------- | -------------------------------------------------------------------- | --------------------------------------------------------- |
| Source    | opaque source reference and purpose                                  | worktree path, Git commit, fingerprint                    |
| Instance  | opaque handle                                                        | ports, caches, paths, Tauri identifier, process ownership |
| Actions   | separate build/test pass or fail and optional failed step            | executable plans and logs                                 |
| Lifecycle | phase, health, staleness                                             | endpoints, windows, jobs, launch commands                 |
| Evidence  | opaque evidence references when a future evidence port supplies them | roots, manifests, filenames                               |
| Judgement | evidence bundle IDs, disposition, findings                           | no runtime or driver authority                            |

`evaluateAgentReviewInstanceStatus` only checks for a current, healthy, running instance. Capability
and attachment discovery belongs to the development adapter; it is not a runtime-facade result.

## Guarantees and exclusions

The accepted facade separates request, build, test, start, status, stop, and recover. It hides the
resolved worktree and allocated resources. It reports semantic lifecycle state and detects stale
state.

This prospective review seam does not claim that stop deletes isolated application state, releases
every endpoint or window, or finalizes a manifest. It does not claim that application composition,
attachment resolution, or durable evidence access exists. Those are future ports and proofs.

The contract contains no Playwright, CDP, DevTools, WebDriver, WDIO, shell command, process ID,
origin, port, window reference, credential, authority secret, or filesystem path. Production is not
given an attachment or evidence route.

## Integration proof still required

Before review/runtime convergence can be called integrated, an application-composed run must:

- resolve an agent-owned worktree to an opaque source reference;
- request, build or test, start, query, and stop/recover one opaque instance through the accepted
  facade;
- let a development-only adapter resolve attachment and evidence access without exposing those
  details to orchestration;
- retain scenario actions, assertions, runtime evidence references, and adapter evidence references
  through an application-owned evidence port;
- prove shutdown and observed resource closure without promising state deletion or manifest
  finalization; and
- prove the attachment, driver, credentials, and production routes remain excluded from production
  composition.

The current three proofs were started by dedicated scripts, so they do not satisfy this integration
gate.
