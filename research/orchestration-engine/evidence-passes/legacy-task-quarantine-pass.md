# Observation pass: legacy Task quarantine

## Anchor

What happens if an older caller invokes the still-registered Task commands, and how much of the earlier product remains behind that boundary?

This pass distinguishes runtime availability, command compatibility, compilation, tests, persistence, and source retention.

## Observed path

1. `active_app.rs` registers nine older Task and Task-run Tauri command names in both release and debug builds.
2. Every handler immediately calls `ensure_legacy_tasks_available()`.
3. That guard always returns `Legacy Tasks are quarantined in the Agent Session reset baseline`.
4. No legacy database is opened and no Git, Codex, validation, or Task mutation occurs.

The commands are therefore deliberately callable but deliberately non-operational.

## Concrete observations

### Quarantine was an explicit product reset

Commit `43a65ed` (`refactor(app): establish the Agent Session reset baseline`, 2026-07-12) made three coordinated changes:

- removed the Legacy Tasks switch and `TaskDashboardScreen` from the product `App`;
- removed Task clients from `main.tsx` composition;
- inserted the fail-closed guard at the start of every legacy Tauri handler.

The commit comment states why the command names remain registered: older callers should receive a deliberate quarantine error. The code does not silently remove or reinterpret those endpoints.

### The complete backend implementation remains behind the guard

`src-tauri/src/lib.rs` still contains the earlier Task backend as one large region before the test module:

- Task, project, repository, branch, worktree, run, artifact, validation, and event DTOs;
- the earlier SQLite schema and migration registry;
- Task dashboard projection and mutation functions;
- repository discovery and worktree registration;
- direct Codex command execution;
- Git diff and validation command runners;
- JSONL parsing, raw-stream retention, post-run capture, and detailed run projection.

The file has 6,275 physical lines; its test module begins at line 4,590 and contains 24 Rust `#[test]` functions. The release command handlers return before entering this implementation, but Rust still parses and type-checks the retained code.

### The legacy database is separate and intentionally untouched

The retained Task path names `codex-orchestrator.sqlite`. The active orchestration application opens `codex-orchestrator-active-v3.sqlite` through `storage.rs`.

Because the quarantine guard runs before `with_app_database`, invoking an older Task command does not open, initialize, or migrate the legacy file. Active-storage tests explicitly verify that opening the current database leaves legacy and prior-v2 files untouched.

This is stronger than UI unreachability: the runtime boundary prevents accidental mutation of retained old data.

### The frontend implementation also remains

The source tree retains:

- `features/taskDashboard/TaskDashboardScreen.tsx` at 1,498 lines;
- Task dashboard, Task-run, validation, local SQLite, Git, Codex, and runtime application/infrastructure modules;
- Tauri clients in `infrastructure/tauriCommands.ts` for the quarantined command names;
- fifteen `App.test.tsx` cases that render `TaskDashboardScreen` directly with fakes.

No non-test production import from `main.tsx`, current `App`, product bootstrap, Agent Sessions, or orchestration features to `TaskDashboardScreen` or the legacy Task clients was found. The TypeScript configuration includes all of `src`, so these files remain type-checked even though they are outside the product entry graph.

### Tests exercise productive-shaped internals, not the release command result

The Rust tests directly call functions behind the guard with in-memory databases and fake runners. They still prove Task durability, repository/worktree anchors, Codex run handling, raw event retention, diff capture, validation capture, and failure behavior.

A separate test proves that the public command guard remains fail-closed. The larger suite therefore validates the retained implementation as an internal historical system while the release boundary exposes only quarantine.

### Command registration alone overstates product capability

These nine operations are present in Tauri command enumeration and have complete DTOs and implementations. None can pass its first application check. A catalogue based only on generated handlers would describe a Task product that does not exist at runtime.

Conversely, describing all code behind the guard as merely dead would omit its continued roles in compatibility behavior, type-checking, regression tests, retained schema knowledge, and historical explanation.

## Unexpected connections

- The older direct Codex/Git/validation runtime remains in the same Rust crate as the newer provider-neutral Agent Session runtime and orchestration execution support.
- Current SQLite connection policy is reused by the dormant legacy database opener even though the active database has a different filename and schema authority.
- The frontend and backend each retain an independently usable Task implementation for tests, but the normal product composes neither.
- Compatibility is preserved at the command-name/error level, not at the old behavior or old-data level.
- Large passing test surfaces can describe an intentionally unavailable product.

## Questions opened by the pass

- Which compatibility consumers, if any, still require the nine registered command names?
- Is preservation of the complete implementation necessary for that compatibility promise, or only preservation of the command shapes and error?
- Which concepts from the Task system remain useful precedents for current orchestration, and which would mislead future development?
- Should historical schema and behavioral tests remain executable in the main crate, move to an archive/reference package, or eventually be removed?
- Are there other quarantined areas where tests prove internal behavior that no product path can invoke?

The pass does not propose removal. It shows that “legacy” currently comprises several different retained values and costs, with a very clear fail-closed runtime boundary.
