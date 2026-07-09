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

## Work Package State

| Package                                 | State                      | Thread  | Integrated commit | Integration result |
| --------------------------------------- | -------------------------- | ------- | ----------------- | ------------------ |
| AS-00 Structural and migration baseline | ready                      | pending | —                 | —                  |
| AS-01 Agent Session contracts           | blocked by AS-00           | pending | —                 | —                  |
| AS-02 Durable repository and queries    | blocked by AS-01           | pending | —                 | —                  |
| AS-03 Real process supervisor           | blocked by AS-01           | pending | —                 | —                  |
| AS-04 Codex CLI runtime adapter         | blocked by AS-03           | pending | —                 | —                  |
| AS-05 Application and Tauri lifecycle   | blocked by AS-02 and AS-04 | pending | —                 | —                  |
| AS-06 Transcript projection and UI      | blocked by AS-05           | pending | —                 | —                  |
| AS-07 End-to-end recovery gate          | blocked by AS-06           | pending | —                 | —                  |

States used: `blocked`, `ready`, `active`, `review`, `correction`, `integrated`, `failed`, and
`superseded`.

## Integration Notes

No work package has started yet.
