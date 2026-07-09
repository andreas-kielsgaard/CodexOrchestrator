# Agent Session Execution Ledger

Started: 2026-07-10

Integration root: current Codex task on `codex/agent-session-reset`

## Integration Protocol

- Each AS work package gets one dedicated Codex work thread and isolated Git worktree.
- Worker threads start only when their declared dependencies are integrated into the root branch.
- Repository planning documents are the shared authority. Custom orchestration skills are not used.
- A worker owns only its work package. It may inspect adjacent code but must not implement later
  packages opportunistically.
- Every worker returns:
  - concise findings and decisions
  - changed-file list
  - validation commands and results
  - remaining risks or follow-ups
  - one scoped commit hash
- Workers do not merge their own branches into the integration root.
- The integration root reviews the commit and boundary fit, cherry-picks or requests correction,
  runs proportionate integration checks, updates this ledger, and only then unlocks dependents.
- If a package changes an accepted architectural decision, the worker must flag it rather than
  silently editing the decision record.

## Work Thread Reasoning Policy

- AS-01 contracts: launched at high before the policy was adjusted; correction work stays narrow.
- AS-02 repository: medium.
- AS-03 supervisor: high because concurrency, cancellation, and shutdown are high-risk.
- AS-04 Codex adapter: medium because the provider protocol is bounded and fixture-testable.
- AS-05 lifecycle integration: high because it joins persistence, processes, and transport.
- AS-06 projection/UI: medium.
- AS-07 recovery gate: medium, focused on verification rather than new architecture.

Medium is the default. Reasoning effort may be reduced for a narrowly mechanical correction, but
should not be raised to compensate for an over-broad prompt. No slice uses xhigh by default.

## Work Package State

| Package                                 | State                      | Thread                                 | Integrated commit | Integration result |
| --------------------------------------- | -------------------------- | -------------------------------------- | ----------------- | ------------------ |
| AS-00 Structural and migration baseline | integrated                 | `019f4905-dc10-7c80-9e42-882196abac18` | `4e171e9`         | accepted           |
| AS-01 Agent Session contracts           | active                     | `019f491f-7f59-7500-9bf0-d0feaa28a59b` | —                 | —                  |
| AS-02 Durable repository and queries    | blocked by AS-01           | pending                                | —                 | —                  |
| AS-03 Real process supervisor           | blocked by AS-01           | pending                                | —                 | —                  |
| AS-04 Codex CLI runtime adapter         | blocked by AS-03           | pending                                | —                 | —                  |
| AS-05 Application and Tauri lifecycle   | blocked by AS-02 and AS-04 | pending                                | —                 | —                  |
| AS-06 Transcript projection and UI      | blocked by AS-05           | pending                                | —                 | —                  |
| AS-07 End-to-end recovery gate          | blocked by AS-06           | pending                                | —                 | —                  |

States used: `blocked`, `ready`, `active`, `review`, `correction`, `integrated`, `failed`, and
`superseded`.

## Integration Notes

- AS-00 started in an isolated Codex worktree from planning baseline commit `625c56a`.
- AS-00 worker commit `21b9ffd` was reviewed and integrated as `4e171e9`.
- Integration verification passed formatting, lint, all 291 frontend tests, 16 focused migration
  tests, and Rust formatting.
- Frontend build and Rust check/test/build remain blocked by confirmed pre-existing task-dashboard
  fixture/projection inconsistencies. The same errors reproduce before AS-00 and are outside its
  changed hunks.
- Generated `storybook-static` output is now ignored by Git, Prettier, and ESLint so preserved local
  output does not contaminate repository validation.
- AS-01 started from integrated root commit `585a1ff` with high reasoning.
