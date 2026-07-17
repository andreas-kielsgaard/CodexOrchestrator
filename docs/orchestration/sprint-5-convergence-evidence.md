# Sprint 5 deterministic convergence evidence

Status: current Sprint 5 convergence record for the accepted non-MCP product state. The minimum
durable initiation/restart path is accepted; MCP correction/integration, external MCP
investigation, and the unresolved Plan Builder MCP failure are deferred to Epic Planner authority
and are not Sprint 5 evidence. Deterministic checks and live observations remain explicitly
separate.

## Accepted current state

The user-authorized scope re-evaluation implemented the minimum truthful initiation path: active-v3
schema version `2` preserves v1 fields additively; native-query/v2 carries the exact proposal
material snapshot plus correlated initiation command, result, event, and provenance; and the
canonical product projection exposes a terminal `initiated` draft with one Epic and ordered
preparatory Sprints. No Work Units or execution are created. WU-S5-17 UI is accepted and requires
no UI/G3 retest.

This accepted state is narrower than broad material generation, artifacts, execution, scheduling,
runners, and continuation, which remain deferred. Tool-specific, Harness, provider/live-proof,
and MCP declarative drift also remain deferred and are not edited by this documentation unit.

## Original deterministic closeout evidence

This section is retained historical evidence from the broader implementation record. Its MCP
transport, tool, and managed-launch checks are not accepted Sprint 5 evidence under decision 0005.

The focused frontend command, native-query, projection, and composition tests passed with the exact
count recorded below after this documentation change. The focused Rust orchestration/library tests
passed with the exact count recorded below. They cover persisted pre-initiation proposal revision,
provenance, strict native-query fixtures/decoding, direct Streamable HTTP authentication/host/Origin
checks, `initialize`/`tools/list`/single-tool schema, revision, and idempotency paths, and the
opt-in managed child launch assembly.

- Frontend: `npx vitest run --maxWorkers=1` over the six cited files: **6 files / 35 tests
  passed**.
- Rust durable/MCP/managed-launch slice: `cargo test orchestration:: --lib`: **13 passed**.
- Rust Codex runtime/argument slice: `cargo test runtime::codex:: --lib`: **20 passed, 1 intentionally
  ignored** (the installed-CLI compatibility probe); Cargo emitted one existing dead-code warning
  for `ProcessSupervisor::system`.

Markdown links, referenced source/test paths, whitespace, and `git diff --check` were checked.

## WU-S5-18 durable initiation and restart evidence

An existing saved proposal was initiated transactionally, then the active database and repository
were reopened. The serialized native-v2 query remained exactly equivalent, retained the initiated draft,
material snapshot, Epic identity, and two ordered preparatory Sprints. A deterministic Rust native-v2
serialization is golden-checked byte-for-byte against the exact initiated-Epic fixture consumed by the
strict frontend decoder; that decoder then composes it through the canonical product read model and the
existing overview adapter. The initiated draft rejects later title edits, cancellation, and managed Plan
Builder re-entry; no additional proposal or execution fact is created.

- `cargo test orchestration:: --lib`: **27 passed**.
- `cargo test --lib`: **127 passed, 2 intentionally ignored probes**.
- Focused frontend native-query/composition/overview checks: **4 files / 64 tests passed**.
- `cargo fmt --check`, touched-file Prettier, `npx tsc --noEmit`, and `git diff --check`: passed.

No paid/live agent prompt was run. This evidence proves durable initiation/recovery and read-side
projection only; it does not prove the later combined live Plan Builder happy path.

## WU-S5-20 non-MCP convergence audit

The audit found and corrected one frontend decoder defect: native-v2 accepted duplicate or orphaned
initiation lifecycle records, which could project an extra accepted Epic from corrupted input. The
decoder now requires one fully consumed command/result/event/provenance/material-snapshot chain per
initiated draft and verifies that it consumes the draft's current proposal. This remains a read-side
rejection boundary; it creates no product facts.

- Focused draft/reconciliation, proposal, strict decoder, canonical composition, overview, and
  initiation UI checks: **5 files / 74 tests passed**.
- The two non-audited suites that failed only in the parallel aggregate run passed serially:
  **2 files / 6 tests**.
- `cargo test orchestration::repository::tests --lib`: **14 passed**.
- `cargo test --lib`: **127 passed, 2 intentionally ignored probes**.
- `npx tsc --noEmit`, frontend lint, frontend production build, `cargo fmt --check`, touched-file
  Prettier, and `git diff --check`: passed.

The default parallel frontend aggregate was not clean: three tests timed out or remained in a
transient clipboard state under load, including one Plan Builder test. Each passed when run
serially. This is historical parallel-run evidence only. Sprint Runner independent follow-up evidence
after WU-S5-20 review completed `npx vitest run --maxWorkers=1` successfully: **83 files / 581 tests
passed** in **194.64 seconds**. Existing React `act` warnings in the draft-reopen test and Node SQLite
experimental warnings remained; no test failed. No paid/live agent prompt was run.

## Aggregate frontend noise

The current aggregate frontend run covered 81 files / 547 tests: 79 files / 545 tests passed, and
two tests reached Vitest's five-second timeout under load: the `App.orchestrations` Plan Builder
source test and the `OrchestrationSection` Plan/Work Unit navigation test. Both files then passed
independently (2 files / 33 tests). This is historical load-sensitive harness timing, not a confirmed
product regression. The later complete serial aggregate passed; see the WU-S5-20 follow-up evidence.

## Historical/deferred explicitly unproven

The user later ran the G3 flow described below, so a successful live flow must not be inferred from
the original deterministic closeout. The observed model did not produce a successful save. Corrected
model skill selection, efficient tool choice, and durable live proposal persistence remain unproven.
No paid/live model prompt was run as part of the correction. Deterministic MCP protocol success
proves the server/handler contract and persisted effect path only. See the historical/deferred
[real-flow gate](sprint-5-extension-and-gates.md#historicaldeferred-real-flow-gate); it is not a
Sprint 5 closure condition.

## User-run G3 failure and correction evidence

This section is retained historical record only. The MCP correction, integration, and external
investigation handback described here are deferred to Epic Planner authority and are not accepted
Sprint 5 evidence.

The user-run Plan Builder transcript at G3 is pre-correction evidence, not convergence evidence. It
records one logical canonical-skill read shown as two lifecycle rows, and two logical MCP operations
shown as four lifecycle rows: one `get_epic_planning_context` call and one
`save_epic_plan_proposal` call. The latter persisted. This evidence does not prove model selection,
live discovery, or the corrected one-call behavior.

The bounded correction exposes one direct typed submission, captures the revision/precondition at
managed invocation start, derives replay protection from the Agent Invocation plus canonical
payload, and projects paired lifecycle rows as one display activity. Harness guidance says discussion
makes zero calls and build/rebuild makes exactly one submission. The repository skill remains a
canonical source that may be loaded once when appropriate; this deterministic evidence does not
claim live discovery or model selection. No paid/live model prompt was run for the correction.

Post-correction validation:

- `cargo test orchestration:: --lib`: **19 passed**.
- `cargo test --lib`: **117 passed, 2 intentionally ignored live/installed-client probes**.
- Focused frontend configuration: **1 file / 2 tests passed**.
- `cargo fmt --check`, `cargo check`, frontend lint, frontend build, touched-file Prettier, and
  `git diff --check`: passed. Cargo check retained the existing non-blocking dead-code warnings.
