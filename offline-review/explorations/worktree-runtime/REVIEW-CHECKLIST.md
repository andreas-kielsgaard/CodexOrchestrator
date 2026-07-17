# Worktree Runtime review checklist

## Offline package

- [ ] Open `worktree-runtime-static-review.html`; confirm it says **static** and **recorded**.
- [ ] Confirm no current-health claim is inferred from the static view.
- [ ] Confirm the current branch/head/dirty state in `evidence-snapshot.json`.
- [ ] Keep the historical two-worktree proof separate from the freshly prepared local manifest.

## Proof acceptance

- [ ] Two worktrees built and ran focused tests against the same committed source.
- [ ] Instance/session/worktree/commit ownership matched for both live status endpoints.
- [ ] Ports, dist, Vite cache, Cargo target, app data, credentials, logs, and evidence roots differed.
- [ ] Stopping `proof-a` left `proof-b` healthy.
- [ ] Final teardown reported zero roots and closed endpoints.
- [ ] Stale recovery evidence is sufficient for exploration, not product process ownership.

## Demonstration tab

- [ ] Identity is inspectable: worktree, build, session, commit, Tauri identifier.
- [ ] Shared-keyed versus isolated material is understandable.
- [ ] Projected, observed, recorded, and unsupported badges are not interchangeable.
- [ ] Lifecycle/health/teardown evidence is inspectable without implying product controls.
- [ ] Unsupported boundaries and human decisions are visible.
- [ ] Direct Tauri-window capture remains a manual gate; indirect 1280×820 evidence is sufficient.

## Accepted corrections

- [ ] Existing manifest is inspected before `prepare` may replace it.
- [ ] Live owned, live unowned, endpoint-only live, and stale states refuse re-prepare.
- [ ] A clean stopped instance may be re-prepared.
- [ ] Git enumerates all untracked files rather than collapsed untracked directories.
- [ ] Nested untracked content changes invalidate the fingerprint.
- [ ] Unreadable or non-regular untracked entries fail closed.
- [ ] Corrections were accepted after Epic review at `4e5027c`.

## Safety

- [ ] Accept the detached-launch-to-manifest-write interval as a prototype-only crash gap.
- [ ] Require a Windows Job Object before product process ownership.
- [ ] Require durable port leases before automatic concurrent launch.
- [ ] Do not claim shared Rust compilation while `sccache` is unavailable.
- [ ] Do not inject provider credentials without a user/policy decision.
- [ ] Do not treat screenshot/recording directories as capture evidence.
- [ ] Do not grow the disposable JS harness into product infrastructure.

## Product decisions

- [ ] Credential policy: ________________________________________________
- [ ] Attention-worthy events: __________________________________________
- [ ] First pause model: ________________________________________________
- [ ] Automatic continuation gates: _____________________________________

## Candidate next slice

- [ ] Focused identity, registry, cache, launch, ownership, health, and recovery modules.
- [ ] Named Job Object with kill-on-close.
- [ ] Durable projected and observed transitions.
- [ ] Explicit idempotent read/start/stop/recover ports.
- [ ] Two-instance, isolated-stop, and restart-recovery proof.
- [ ] Scheduling, automatic approvals, credentials, capture, and full pause/resume remain deferred.

Decision: [ ] accept exploration [ ] request correction [ ] defer

Notes:

---

---
