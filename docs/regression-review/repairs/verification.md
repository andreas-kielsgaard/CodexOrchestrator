# Repair verification

Checked on 7 September 2026 in `codex/session-event-regression-review`. These checks cover the repair changes on base `e77a725`, before consolidation into Harness UX. They do not establish a published application build or user acceptance.

| Check                        | Result                                  | Limit                                                                                                                                                |
| ---------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Full frontend suite          | **997 passed, zero failed**             | Includes fake-client and JSDOM tests. Not native execution.                                                                                          |
| New joined Rust repair tests | **9 passed**                            | Real repositories, application services and local HTTP proxy; recording inference runtime and in-process sidecar transport.                          |
| Full Rust library suite      | **711 passed, one failed, two ignored** | One existing orchestration MCP test timed out while the full suite ran.                                                                              |
| Isolated rerun of that test  | **Passed**                              | `work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness`; about 61 seconds. No product changes were made to hide the timeout. |
| TypeScript + Vite build      | **Passed**                              | Existing large-chunk warning remains.                                                                                                                |
| Native `cargo build`         | **Passed**                              | Not a native-window walkthrough. Unused-item warnings remain.                                                                                        |
| Four-width browser run       | **Passed; zero page errors**            | Actual React/CSS, fake clients. See `evidence/results.json`.                                                                                         |
| Changed frontend lint        | **Passed**                              | Limited to changed/new source files; not a repo-wide lint cleanup.                                                                                   |
| `git diff --check`           | **Passed**                              | Whitespace only.                                                                                                                                     |

The full Rust command used `--profile test-fast --lib -- --skip runtime::codex::tests::installed`; it reported zero filtered tests. The `live-tests` feature was not enabled. The two ignored cases are existing process/helper proofs. The separate nine-test command was:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib repair_tests
```

The recorded full-suite timeout was a `reqwest` body-read timeout at `orchestration/bootstrap_transition.rs:6430`. Its isolated rerun used `--test-threads=1`. Passing that rerun does not turn the earlier full-suite result into a clean pass.

## Evidence boundaries

The Rust fixture uses the same creation resolver, Session application, directory adapter, Workflow compiler/executor, SQLite stores and MCP binding/proxy that the native composition mounts. It replaces inference with a recording runtime. The completion test enters through the normal persisted terminal-notification callback. The MCP test sends real local HTTP requests through the proxy to the new receiver.

The browser fixture mounts the product App and feature components. Its instance persistence is local fixture storage. Native durability is covered separately by SQLite reopen assertions. Browser drag, connection save, collapse/layout, instance creation, Session opening, Back and saved-instance reopening are checked. Screenshots were also inspected for layout errors.

No live provider request, native-window create/reopen walkthrough, old-data migration, release package, user acceptance or parallel-branch integration was performed. The existing `regression-review` runtime manifest is stale and was not launched as the repaired build.

## Harness UX consolidation

On 7 September 2026, repair commit `c1b89b9` was fast-forwarded into `codex/harness-ux-workflow-convergence` in the existing Harness UX worktree. There were no conflicts or product-code changes during the merge.

Checks repeated from that worktree passed: **997 frontend tests**, **9 joined Rust repair tests**, and the **TypeScript/Vite build**. The old dependency link was refreshed, and relocated Tauri build-cache entries were rebuilt because they contained the previous worktree path. No source fix was needed.

This rerun does not replace the full Rust suite result above or establish a mounted native application. Main and unrelated development branches were not merged.
