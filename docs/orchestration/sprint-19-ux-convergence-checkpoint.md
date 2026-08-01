# Sprint 19 UX convergence checkpoint

## Review surface

Run `npm run dev -- --host 127.0.0.1 --port 51919`, then open
`http://127.0.0.1:51919/?recorded-plan-builder`.

- Open `Codex Epic Runner workspace development`, then `Product decisions`, to review the distinct single-column Epic view, persistent Epic identity, explicit decision hierarchy, secondary review/change/conflict detail, and exact-passage read-only Agent Session citation popup.
- Return to `Plan` to confirm that Plan remains a separate Epic view.
- From the orchestration overview, open `Plan an Epic` to review Agent identity inline in the canonical Session title and Agent attribution. The proposal retains its single visual identity marker; the Session does not add a duplicate marker.
- Open the recorded Sprint, Work Slice, and Work Unit paths to confirm that `Review files` remains in the persistent Sprint header.

This route is a development-only recorded demonstration. It does not grant product or production authority.

## Checkpoint status

Integrated in this checkpoint:

- the accepted Agent identity Session-layout correction from `06f04c43c82dd7ef1146bee427ec764cb3852d2f`;
- the accepted Product Decisions exploration from `5ed3b496f5016b39e6dd0b5b938f05b5aac02820`;
- current-main navigation and Batch 18 contextual File Review behavior from base `ee80dc9b8da96a3e3e2d44293c9a3684d746e525`.

Provisional:

- the typed Product Decisions graph, validation rules, and Epic-scoped read boundary;
- the hierarchy, evidence, candidate, conflict, and compliance-request presentation model.

Recorded-only:

- Product Decisions data and exact-passage Agent Session evidence;
- the compliance-review request and all development demonstration state.

Still unproven:

- production persistence, lookup, extraction, compilation, reconciliation, mutation, conflict resolution, origin resolution, graph API, or audit execution;
- live native or visual/HIL behavior, exhaustive keyboard/dialog accessibility, release File Review availability, user review, acceptance, promotion, merge, publication, or production readiness.

## Automated evidence

- focused integration: 9 files, 86 tests passed;
- full frontend: 110 files, 729 tests passed;
- TypeScript/Vite build: passed, 2,079 modules transformed;
- ESLint: passed;
- recorded development route: Vite served HTTP 200 with the application root;
- changed-path Prettier check: passed;
- repository diff check: no whitespace errors.

The suites cover Product Decisions source exclusion from product boot; Plan, Harness, Agent Session, File Review, and Sprint navigation; inline Agent identity; the read-only citation popup; and persistent Sprint-header File Review across Sprint, Work Slice, and Work Unit routes. Passing React tests still emit existing `act(...)` warning noise; no failure is hidden by those warnings.

The repository-wide Prettier check still reports 33 pre-existing baseline files outside this slice.
