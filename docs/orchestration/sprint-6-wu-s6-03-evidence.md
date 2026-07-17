# WU-S6-03 evidence

## Implemented

- Button and agent requests enter one typed application controller and one accessible in-app confirmation modal. Duplicate request IDs are ignored; distinct requests are serialized.
- Only explicit confirm or reject resolves the backend request. Raw Tauri invoke/listen remains in infrastructure adapters; the removed direct-initiation command and `window.confirm` are absent from production frontend code.
- Button-origin projected initiation schedules a durable managed-session context delivery. Its claim persists a preallocated stable Agent Invocation ID and the idempotent application send uses that exact identity without changing submitted text or provenance.
- Runtime start/resume success records a separate durable launch-acceptance fact. Restart reconciliation requires that fact plus the expected session and application provenance; `started_at` alone remains unaccepted and retryable. Accepted claims consume once without redelivery, consume failure converges on reopen, and reconciliation is repeatable. Agent-origin initiation schedules none.
- Strict transition-v2 decoding joins through `productReadModelComposer` and presents preparation, Bootstrap attempt/lifecycle, retry/blocked, material acceptance, and Runner launch states without claiming Sprint start.
- Refresh failures replace prior orchestration success with unavailable state. A successful confirmation remains successful and is never resolved twice when the later application refresh fails.

## Deterministic validation

- Prior accepted frontend correction: 87 files, 601 tests passed; frontend was not touched or rerun for this backend-only correction.
- Rust library: 165 passed, 3 ignored paid/live probes, 0 failed.
- Correction focus: 15 Agent Session application, 5 Agent Session repository, 10 storage/migration, and 8 orchestration application tests passed; 1 paid live orchestration probe remained ignored.
- Prior TypeScript, ESLint, and production build plus current `cargo check --lib`: passed. Existing dead-code warnings remain in Rust; ESLint has no warnings.
- Production scan found no `initiate_epic` or `window.confirm` use.

## Live boundary

No live paid prompt or final production confirmation click was performed. Actual application components and production adapters are covered deterministically. The boundary where a runtime returns success but the durable acceptance marker fails to persist cannot prove atomic external provider processing and is conservatively retryable; live provider behavior and a human confirmation click remain unproven.
