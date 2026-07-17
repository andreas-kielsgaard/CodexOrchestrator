# Sprint 6 convergence evidence

Status: **partial**. Deterministic convergence and isolated installed-Codex Plan Builder,
Bootstrap Generator, and Epic Runner paths pass. The production user-confirmation click and the
integrated post-click chain remain a manual gate; no confirmation was bypassed.

## Independent corrections

- User-authored managed Plan Builder queries retain `user` transcript provenance, while the
  product Plan/Rebuild action retains `application` provenance. An optional confirmed-initiation
  prefix remains application context and does not rewrite either action's submitted text or
  provenance. The prior blanket application provenance kept the real `Plan Epic` control disabled;
  the later blanket user correction briefly mislabeled the product Plan/Rebuild prompt.
- Proposal saves start an immediate SQLite transaction. This removes the WAL deferred-read to
  writer-upgrade race reproduced live as `record applied proposal command: database is locked`.
  A 30-second busy-timeout experiment did not remove the first failed call and was reverted.
- Bootstrap and Runner `launchedAt` facts now require the durable provider-neutral launch-
  acceptance marker. Persisted-but-unaccepted invocations remain conservatively unlaunched.

## Deterministic proof

- Rust library, serial: **168 passed, 6 intentionally ignored, 0 failed**.
- Focused after corrections: Plan Builder application **9 passed, 3 live ignored**; transition
  **17 passed, 1 live ignored**; orchestration repository **15 passed**; launch-acceptance crash
  windows **2 passed**.
- Installed CLI help/capability probe: **1 passed**.
- The earlier frontend serial aggregate passed **87 files / 601 tests**. After the provenance
  correction added one test, the current isolated-worker aggregate passed **87 files / 602
  tests**; the four focused provenance/UI files passed **51/51**. A forced shared single-fork run
  reproduces pre-existing cross-file DOM cleanup noise. TypeScript, ESLint, production Vite build,
  and current Tauri release build passed. Existing React `act`, Node SQLite experimental, Rust
  dead-code, and Tauri bundle-identifier warnings are non-failures.
- Rustfmt, the touched evidence file's Prettier check, and `git diff --check` passed. The repo-wide
  Prettier check still reports **39 pre-existing files** outside this unit's changes.
- Schema-v7 migration/reopen, native-v2 Rust/TypeScript goldens, strict transition-v2 decoding and
  canonical composition, confirmation controller/modal, one-shot context crash windows, Bootstrap
  retry/cross-attempt/path containment, and exact downstream launch counts pass in the aggregate.

## Installed-Codex proof

- Fresh ordinary discussion: **1 user-origin invocation, 0 proposals, 0 semantic calls**.
- Fresh build plus managed resume/rebuild: **2 user-origin invocations, 2 proposal revisions,
  2 `submit_epic_plan_proposal` calls, 0 initiation calls**; reopen projected both revisions.
- Failed evidence was retained during investigation: one rebuild run stopped at one proposal after
  the safe SQLite `internal_error`; the busy-timeout experiment reached two proposals only after
  **3** submit calls including a failed retry. The immediate-transaction correction produced the
  final exact **2/2** result.
- One isolated, test-owned already-confirmed initiation used the production specialized service,
  production MCP adapter, and real Codex. It observed **2 completed role invocations**, exactly
  **1** `complete_epic_bootstrap` call, **1** semantic fact, **2** application-written material
  files with matching paths/sizes/SHA-256 values, **1** same-attempt accepted inventory, and
  exactly **1** launch-accepted read-only Runner. Runner exposed no MCP product action.
- That isolated state contained the one planned `initiated_sprints` row required by durable
  initiation, but **0 Sprint Agent Sessions, 0 Sprint actions, 0 Work Units, and 0 execution
  effects**.

## Production UI and residual boundary

The current release application loaded through real Tauri composition. It rendered the active
pre-initiation draft and the correlated transition label `Bootstrap attempt 1 running`, while
truthfully showing that no Sprint execution or Epic lifecycle had been observed. The retained
failed draft visibly exposed the provenance defect and disabled plan action before correction.

The corrected release binary built successfully, but Windows locked before it could be reinspected.
No modal confirmation was clicked. The remaining integrated proof is: unlock Windows, issue one
new user-origin discussion/build query on the real draft, open the shared initiation modal, and let
the human confirm or reject. Only after confirmation may the application demonstrate the integrated
prepared Bootstrap-to-Runner chain. Filesystem link/reparse checks remain deterministic rather than
race-proof, confirmation registrations remain process-memory state before decision, and no provider
process reattachment or atomic external-provider-processing claim is made.
