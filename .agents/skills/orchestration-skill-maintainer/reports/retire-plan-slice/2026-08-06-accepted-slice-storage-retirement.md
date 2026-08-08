# Accepted Slice storage retirement

## Observation

The workflow exhausted the disk again after the first retirement revision. It retained 42 registered worktrees, but source checkouts were not the main cost. Old controlled-live and validation routes contained hundreds of gigabytes of duplicated Cargo output. One retained PIP-02 route held about 123 GB across nine runtime packages; `C:\crp` held another 122.6 GB of marked Cargo targets. The active NCHP source route also accumulated several superseded evaluation builds.

Scoped `cargo clean` operations preserved source, commits, databases, recordings, manifests, credentials, and active runtime state while increasing free space from 9.9 GB to 301 GB.

## Theory

The first revision established safe retirement after Slice acceptance, but two gaps remained:

- pre-launch storage judgment relied on current headroom rather than the likely peak of concurrent isolated builds and runtimes; and
- retaining a route for task or evidence continuity was still treated as a reason to retain every reproducible candidate build inside it.

The Overall Plan had already described continuous hygiene, yet reclamation did not execute before new artifact-heavy work. Worktree release also lacks a clearly evidenced host lifecycle for many completed tasks, which makes retaining registrations reasonable but does not justify retaining their generated output.

## Revision

- `run-overall-plan` now measures headroom against the ready packet's likely concurrent artifact footprint and reclaims accepted generated state before launch.
- `evaluate-plan-slice` completes eligible reclamation before starting newly unlocked artifact-producing work.
- `retire-plan-slice` separates route retention from reproducible-output retention, including superseded candidate copies inside retained routes.
- `execute-plan-step` reclaims superseded reproducible validation/runtime copies while preserving useful caches, continuation state, and non-reproducible evidence.

## Evaluation

This keeps cleanup with the roles that can act on it: the Plan Step owns its generated copies, and the Overall Plan owns accepted-route retirement. It does not require canonical merge before reclaiming reproducible output, weaken provenance, or authorize broad worktree deletion. The wording may preserve more state when evidence needs are unclear, but it should prevent route retention from multiplying build output indefinitely.
