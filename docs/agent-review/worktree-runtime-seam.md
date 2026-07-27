# Worktree review instance seam

## Status

This branch defines the review-side application contract. It does not implement the worktree
runtime or merge its exploration branch. The read-only comparison used
`codex/explore-worktree-runtime` at `4e5027cc1fe48dee830ad5a3f7e61d84ef664f48` on 2026-07-27.

The contract is in `src/application/agentReview/worktreeRuntime.ts`. It assumes every agent works in
a named Git worktree and keeps driver protocols behind review adapters.

## Semantic flow

1. The application requests `build-and-launch-worktree-review-instance` with the expected worktree
   path, Git commit, source fingerprint, development/test mode, required capabilities, isolation,
   and mandatory cleanup.
2. The runtime verifies identity, compiles the worktree, allocates isolated state and ephemeral
   ports, launches the instance, and owns its processes and windows.
3. The runtime returns instance/build/session identity, an owned HTTP endpoint and/or opaque native
   window reference, semantic capabilities, and runtime/review evidence roots.
4. A review adapter validates the result, resolves its own driver-specific attachment, executes one
   bounded scenario, and writes evidence under the returned review root.
5. Review evidence links the instance ID and runtime manifest. A separate agent judgement consumes
   evidence; it does not gain runtime or driver authority.
6. The application asks the runtime to stop the named instance. The stop result records endpoint and
   window release, isolated-state removal, and finalized runtime evidence.

## Request and result boundary

| Concern             | Required semantic field                                                | Owner                                     |
| ------------------- | ---------------------------------------------------------------------- | ----------------------------------------- |
| Source identity     | worktree path, Git commit, source fingerprint                          | application requests; runtime verifies    |
| Build identity      | instance, session, build, Tauri identifier                             | runtime                                   |
| Isolation           | isolated app data, scrubbed credentials, ephemeral ports               | runtime                                   |
| Lifecycle           | build/launch/read/stop with required cleanup                           | runtime                                   |
| Renderer access     | runtime-owned HTTP origin                                              | runtime result; renderer adapter consumes |
| Native access       | opaque runtime-owned window reference                                  | runtime result; native adapter resolves   |
| Capability exposure | renderer endpoint, owned window, inspectable WebView, native IPC, logs | runtime observes                          |
| Evidence            | runtime root/manifest and separate review root                         | runtime allocates; adapter appends        |
| Judgement           | evidence bundle IDs, disposition, findings                             | review boundary                           |

`evaluateAgentReviewInstance` rejects stale source identity, wrong mode, missing required
capabilities, absent endpoint/window references, or missing evidence roots before an adapter runs.
Production is not an accepted instance mode.

## Driver and authority boundary

The runtime contract contains no Playwright, CDP, DevTools, WebDriver, WDIO, shell command, process
ID, or raw native invocation field. Runtime implementations may prepare an inspectable development
window, but the chosen protocol and commands remain inside a review adapter. Orchestration sees only
neutral requests, evidence, and judgement.

An endpoint or window reference is scoped to its runtime-owned instance and expires when that
instance stops. It grants no permission to attach to unrelated processes, mutate orchestration
state, read ambient credentials, or inspect production data.

## Evidence and cleanup gates

Before this seam can be claimed as integrated, one application-originated run must retain:

- the request and verified worktree/build identity;
- runtime manifest with observed build, launch, port/window ownership, and capability state;
- adapter manifest linked to the same instance ID;
- scenario actions and assertions appropriate to its lane;
- stop result proving endpoint/window release and isolated-state removal;
- a production-exclusion scan showing that review routes, permissions, and adapters are absent.

The current three retained proofs satisfy their lane-specific claims, but were started directly by
dedicated scripts and therefore do not satisfy this integration gate.
