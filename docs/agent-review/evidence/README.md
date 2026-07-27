# Retained agent-review evidence

These files are the approved subset copied by `npm run review:retain-evidence` after:

- `npm run review:renderer`
- `npm run review:native-attach`
- `npm run review:native`

Each lane's manifest records the revision, branch, worktree, application mode, driver, platform,
scenario, assertions, files, disposition, and unverified claims. Screenshots are observations; use
the paired assertions, semantic snapshots, traces, lifecycle records, and logs for behavioral
claims.

The retained set excludes WebView2 profiles, application databases, active endpoint files, and
credentials. Generated working state remains ignored under `.dev/agent-review/` or `test-results/`.

These proofs predate application integration with the worktree-runtime lifecycle port, so their
manifests do not claim a runtime-owned instance handoff. The required linkage is documented in
`docs/agent-review/worktree-runtime-seam.md`.
