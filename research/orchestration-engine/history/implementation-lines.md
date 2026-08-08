# Material implementation lines

## Shared point

The three latest substantial product lines observed during preparation share commit `e3bde2c` (`Keep recorded orchestration views renderable`) and then diverge. The divergence is architectural evidence: later work was developed and published in parallel rather than accumulated on one canonical branch.

## Operational, native-profile, and MCP line

- Initial research tip: `b28137b`; current inspected tip: `9240364`
- Commits after shared point at current tip: 72
- Main areas: orchestration lifecycle corrections, Handler/Implementer execution, runtime and MCP scoping, owned WebView review control, native Codex profiles, safety/readiness, and native MCP reporting.
- Current continuation: `dbe321d` adds three native-profile MCP selection/probe serialization commits after the initial anchor. `9240364` then binds the shared Agent Session application to a selected, ready Native Profile. A parallel reconstructed MCP line ends at `385a4db`; it has different ancestry but represents the same three-commit correction sequence at a high level.

This is the largest and newest clean operational sample, but it does not include the sibling Product Decisions or final-settlement trees.

## Product Decisions, navigation, and inspection line

- Tip: `82d9351`
- Commits after shared point: 37
- Main areas: Work Unit Activity and Evidence, Agent Session turn inspection, typed navigation and exact returns, command bar, Product Decision evidence/history, durable versions, correction conversations, proposal acceptance, and live guards.

This line adds seven Product Decision Tauri operations and a large frontend product area. It predates the native-profile additions on the operational line.

## Epic continuation and final-settlement line

- Tip: `8965191`
- Commits after shared point: 36
- Main areas: durable Sprint continuation settlement, dependency waits, decision reconciliation, Epic receipt of Sprint results, successor realization, and exact final Epic settlement projection.

This line is concentrated in orchestration persistence, transition services, native-query contracts, and Epic/Sprint presentation.

## Earlier retained lines

Archive, explore, checkpoint, review, and correction branches remain useful for reconstructing intent and alternatives. Their names are clues, not disposition evidence. In particular:

- `codex/archive-*` preserves pre-reset or experimental foundations.
- `codex/explore-*` contains File Review, Harness, worktree-runtime, testing-feedback, and Product Decision explorations.
- `codex/integration-*` captures staged convergence points not necessarily present on `main`.
- Accepted/correction branches preserve evidence for individual operational boundaries.

## Research use

The investigation will compare capabilities and final trees across these lines. It will not create a synthetic aggregate merge before understanding whether conflicting artifacts are cumulative, alternative, superseded, or still active.
