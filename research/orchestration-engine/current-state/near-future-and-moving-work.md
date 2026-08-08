# Near-future and moving orchestration work

This view separates implemented work near the stable `9240364` snapshot from behavior already treated as current. “Near future” here means locally present descendant, sibling or uncommitted implementation evidence. It does not promise that the work will be integrated or released.

## Direct descendant now locally present

| State | Location | What it adds |
| --- | --- | --- |
| `7fd921a` | detached `af29` history | partial Handler execution-support recovery |
| `64162ca` | detached `af29` history | recovery-path proof |
| `550ddc5` | detached `af29` history | Handler execution-support recovery across worktrees |
| detached committed tip `63386e6` | worktree `af29` | durable projection of Handler activation failures before application readiness |
| uncommitted change on `63386e6` | `af29/src-tauri/src/worktree_review/catalog.rs` | draft resilience when registered review worktrees are unavailable or do not share repository identity |

This four-commit sequence is a direct descendant of `9240364`. It appeared after the stable evidence passes and has not been merged into the research branch.

## Substantial sibling product lines

| Tip | Product addition | Relationship to the stable snapshot |
| --- | --- | --- |
| `82d9351` | Product Decisions, version inspection, correction authority and navigation | sibling line after the shared operational foundation |
| `8965191` | Sprint continuation and strict final Epic settlement | sibling line after the shared operational foundation |

Both look cumulative enough to matter to the near-future product picture, but neither can be described as integrated current behavior. Their ancestry and contents are recorded in [material implementation lines](../history/implementation-lines.md).

## Uncommitted evidence still worth retaining

- an early Work Slice Planner prelaunch precursor, later absorbed and extended by committed work;
- a stale-base runtime/toolchain alternative with some unique developer-test behavior;
- deterministic presentation and review Harness alternatives;
- a mixed Harness relocation in dirty local `main`;
- the active `af29` Worktree Review catalog-identity resilience draft.

These states have different meanings. They should not be combined into an “unfinished features” bucket. Exact paths and counts are maintained in the [inspection register](../research-context/inspection-register.md); behavioral interpretations are in the [moving-state evidence passes](../evidence-passes/README.md#moving-working-state-passes).

## Near-future convergence questions

The nearby implementation makes these questions consequential:

- Which line should become the product integration authority?
- Should Product Decisions and final settlement join the operational spine before further scope grows?
- Is the compile-time application checkout a temporary productive adapter or the intended repository-selection model?
- Should Native Profile identity binding expand into one shared execution-policy contract?
- Which automatic lifecycle and recovery facts need a fresher or clearer frontend projection?
- Which internal review capabilities should remain verification infrastructure versus become supported operator product?

These questions inform inspection and integration. They are not gates that every future agent must resolve before making progress.

## Snapshot handling

This document is a concise view of work locally present at the 2026-08-07 20:50 +02:00 refresh. When nearby work settles, revise the current/near-future distinction rather than accumulating an indefinite chronological log here. Commit history and the inspection register retain the detailed lineage.
