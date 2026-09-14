# Legacy task implementation retirement

Status: complete; implemented and validated.

Branch: `cleanup/quarantined-legacy-task-code`.

Worktree: `C:/Users/user/.codex/worktrees/legacy-task-retirement`.

Inspected on 2026-09-14 at `e2bfc6c`. The existing edit to
`docs/agent-session/repository-session-navigation-plan.md` is outside this work.

## Outcome

Remove the retired Task/TaskRun/dashboard implementation from source, command registration,
tests, and current architecture guidance. Keep the current Agent Session, orchestration,
Workflow, repository, and Worktree Review implementations intact.

No database conversion or application-data deletion is needed for this source retirement.
The current application opens `codex-orchestrator-active-v3.sqlite`; its storage tests already
verify that the original task database and active-v2 database remain untouched.

## Evidence

- `active_app.rs` opens the current database through `product_database::open`.
- The nine legacy task commands reject before opening the old database or running a process.
- The only observed external references to legacy items in `lib.rs` are its ten command
  registrations, including the unused `app_metadata` command.
- A TypeScript import trace, including type imports and dynamic imports, identified 101 frontend
  files in the legacy dependency group: 53 implementation files and 48 test files, approximately
  27,000 lines. None is reachable from the normal app or Agent Session harness entry point.
- `src/app/App.test.tsx` tests the quarantined dashboard. Current app tests are separate.
- The legacy physical-worktree adapter is consumed only by the legacy runtime composition and
  its tests. Removing it does not require removing the current Rust worktree service.

These source findings defined the implemented deletion set.

## Delete

### Frontend directories, including their tests

| Directory                        | Files |
| -------------------------------- | ----: |
| `src/domain/`                    |    27 |
| `src/features/taskDashboard/`    |     1 |
| `src/infrastructure/sqlite/`     |    32 |
| `src/infrastructure/codex/`      |     4 |
| `src/infrastructure/git/`        |     7 |
| `src/infrastructure/validation/` |     2 |

### Individual frontend files

Delete `src/app/App.test.tsx`.

Delete these modules under `src/application/`, with each corresponding `.test.ts`:

- `diffCollection.ts`
- `postRunCaptureComposition.ts`
- `repoRegistryScan.ts`
- `runComposition.ts`
- `taskDashboardClient.ts`
- `taskRunDetailClient.ts`
- `taskRunLifecycle.ts`
- `taskWorktreeSelection.ts`
- `validationCommandRunner.ts`

Also delete `src/application/runtimeCommandClient.ts`, which has no separate test file.

Delete these modules under `src/infrastructure/`, with each corresponding `.test.ts`:

- `localRuntimeCommands.ts`
- `localRuntimeComposition.ts`
- `tauriCommands.ts`
- `tauriWorktreeApplication.ts`

### Rust implementation and command registrations

Reduce `src-tauri/src/lib.rs` to its current module declarations and the two public entry points:
`run` and `run_harness_engine_sidecar_if_requested`. Delete its old DTOs, task services, process
runners, Git helpers, database opener, migration ledger, schemas, and legacy tests.

Remove these entries from `active_app.rs` and their implementations:

- `app_metadata`
- `load_open_task_dashboard`
- `register_task_worktree`
- `register_task_repo`
- `discover_task_repos`
- `create_open_task`
- `update_open_task`
- `archive_open_task`
- `load_task_run_detail`
- `start_codex_task_run`

Remove `quarantine_archived_prototype_tables` and its four prototype-table constants from
`agent_sessions/repository/schema.rs`, plus its re-export in `repository/mod.rs`. Its only caller
is the old migration registry being deleted.

## Edit and retain

- Remove dashboard-only selectors from `src/styles.css` after checking retained component uses.
  Shared controls and layout styles need a selector-level check.
- Keep the Agent Session schemas, `ensure_agent_session_ownership_schema`, and `table_columns`.
  The ownership migration still uses that helper.
- Keep `storage.rs`, `product_database`, and `persistence`, including active-v3 migrations and
  the tests for old-file preservation and current Session migration/reopen behavior.
- Keep current Rust runtime, Git/repository, and Worktree Application services. Similar names
  in the old frontend do not make these services obsolete.
- The separate disconnected-code cleanup removed the unused runtime-status/widget frontend and
  old Workflow components in commit `67b325a` while this work was running. Those changes are
  outside this retirement commit; its optional review server remains.
- Update `README.md`, `docs/architecture.md`, `docs/agent-session/README.md`, and
  `src-tauri/AGENTS.md` where they describe retained legacy code or direct work into deleted
  modules. Mark `docs/agent-session/prototype-database.md` as historical. Historical logs remain
  historical records.

No new compatibility module, replacement task abstraction, or extraction is currently indicated.

## Implementation order and verification

1. Recheck the deletion set against the current checkout, then remove the frontend group, Rust
   implementation, obsolete commands, and prototype-migration helper together.
2. Remove task-only CSS and update the directly affected current documentation.
3. Check for surviving imports and command references. Run the frontend build, full frontend
   suite, lint, Rust check, full Rust library suite, and capability integration test. Check
   formatting and the diff for the changed files.
4. Run the existing file-backed database preservation and Session migration/reopen tests as
   part of that validation. A lower test count is expected because retired behavior loses its
   tests; preserve coverage of current behavior.
5. Build and launch the desktop app. Check startup, navigation, stored Session history,
   configuration, Workflow, and repository/Worktree Review browsing. Inspect the affected
   shared controls visually. These checks need no paid provider invocation.

Completion means the retired task implementation is absent, current application behavior still
passes its checks, and current guidance no longer presents the removed stack as available.

## Execution record

Implemented on 2026-09-14 in the worktree above. Removed all 101 identified frontend files and
129 task-only CSS selectors. Reduced `src-tauri/src/lib.rs` from 6,293 lines to 33 lines. The
current database implementation and its upgrade/preservation coverage remain.

Validation:

- Frontend build and full suite: 681 tests in 122 files passed after the separate cleanup commit.
- Full Rust library coverage: all 695 tests passed. The first 278 results were retained from the
  serial run; the remaining 417 passed in concurrent batches, with no missing test names.
- Rust check, capability-boundary integration test, and desktop debug build passed.
- Source lint (`eslint src`), formatting of changed files, and `git diff --check` passed.
- Full-project lint found 35 errors and one warning in unchanged historical browser-review
  scripts. Full Rust formatting also reports existing differences outside the edited files.
- Launched the built desktop executable with disposable app data. Inspected Orchestration,
  Workflow, Capability Profiles, Agent Sessions, and Worktree Review. Registered the local
  repository and browsed this branch and its worktree. Shared controls remained readable.
- Reopened a seeded completed Session in the desktop UI; its saved prompt and final response
  remained readable without a configured provider. A further restart preserved the fixture
  records and database integrity. Both older database sentinel files remained byte-identical.
- The original checkout's three pre-existing documentation files remain byte-identical to the
  recorded preservation hashes.

No provider invocation was needed for the desktop smoke check. Local logs, test manifests,
screenshots, and disposable data are retained under `.dev/legacy-task-retirement/` and are not part
of the source commit.

Follow-up: [real agent verification](legacy-task-retirement-verification.md) records authenticated
conversation, file-tool, cancellation, and continuation checks, plus the requested code analysis
of the remaining impact. Cancellation succeeded with a roughly 20-second delay requiring separate
investigation.
