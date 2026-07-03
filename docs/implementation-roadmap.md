# Codex Orchestrator Implementation Roadmap

Date: 2026-07-02

This roadmap describes the product direction and the next implementation layers. For the detailed
near-term worker sequence, use `docs/first-slice-completion-plan.md`.

## Product Goal

Build a local-first control plane for Codex-driven work. Codex remains the execution engine; this
app tracks tasks, repos, worktrees, runs, conversations, artifacts, validation results, and review
state so the user can answer:

- What needs my attention?
- Where does this work live technically?
- What happened during the run?
- What should I review or do next?

## Core Model

- `Task`: the unit of attention.
- `Project`: the human purpose boundary.
- `Repo`, `Branch`, `Worktree`: technical anchors.
- `Conversation`: reasoning or execution provenance.
- `TaskRun`: one attempt to do a task.
- `Artifact`: durable output such as final response, raw JSONL, diff, validation log, note, or
  screenshot.
- `ValidationRun`: one validation attempt and its outcome.
- `Event`: append-only audit/history record.

Keep these concepts separate. The dashboard should be task-centered; technical drilldowns should
explain where work lives.

## Current Reality

Already built on `main`:

- Tauri v2 + React + TypeScript + Vite skeleton.
- Seed Open Tasks dashboard.
- TypeScript domain model and dashboard projection.
- Pure TypeScript Git output parsers and repo sync planning/application logic.
- SQLite schema, migrations, and store adapters for repo sync, open tasks, events, task runs,
  conversations, artifacts, validation runs, and the app store bundle.
- Application-level task-run lifecycle recorder over existing store boundaries.
- Codex JSONL parser boundary for captured `codex exec --json` streams.
- Runtime-facing local SQLite database opener over the app store bundle.
- Codex exec runtime adapter for non-interactive JSONL runs.
- Application-layer run composition service over injected stores and Codex runtime.
- Application-layer repo registry scan service over injected Git scanner and repo sync store
  boundaries.
- Application-layer task worktree selection/creation service over injected repo scan, task stores,
  and Git worktree creator boundaries.
- Application-layer Open Tasks dashboard client and React/Tauri command boundary for
  load/create/update/archive, with the visible dashboard no longer importing seed data directly.
- Rust-side SQLite backend for the Open Tasks Tauri commands, choosing the app data database and
  returning the existing dashboard snapshot shape.
- Application-layer diff collection service over injected stores and `GitDiffProvider`.
- Application-layer validation command runner service over injected stores and command runtime.
- Node-side validation command runtime adapter over `child_process.spawn`.
- Node-only local Git runtime adapters for repo scanning, worktree creation, and tracked-file diff
  collection.
- Node-only local runtime service composition that opens the app SQLite database once, reuses the
  store bundle, and wires the merged application services to concrete local Git, Codex, and
  validation adapters.
- Browser-safe runtime command contract and Tauri `invoke` client for `start_codex_task_run`, plus
  Open Tasks run-control UI that calls the injected runtime client for tasks with worktree paths.

Important gaps:

- No Rust/Tauri backend command for `start_codex_task_run`, so WebView run controls cannot yet
  execute live Codex runs.
- No repo/worktree UI that injects the local Git-backed runtime composition into user flows.
- No task/run detail view.
- No runtime triggers for post-run diff/validation artifacts.
- The Rust/Tauri backend is not fully compile-verified locally. Rust/Cargo are installed and
  `cargo metadata` succeeds, but `cargo test`, `cargo build`, and `npm run build:tauri` currently
  fail because the MSVC linker `link.exe` is unavailable.

## Guiding Decisions

- Keep Codex credentials owned by Codex. Do not read or manage them directly.
- Put Codex execution behind a runtime adapter boundary.
- Put Git execution behind adapter/command boundaries; React should not run Git directly.
- Keep SQLite behind store boundaries and app-level wiring.
- Store raw run output before trusting normalized summaries.
- Make workflow behavior data-driven over time, but do not build a workflow engine before one real
  task-run loop works.

## First Usable Slice

Definition: a user can create or select a task, connect it to a repo/worktree, start a
`codex exec --json` run, preserve raw output, update task/run state, and review final response,
events, diff, and validation result in the app.

Current planned sequence:

1. Add the Rust/Tauri backend command for `start_codex_task_run`.
2. Add task/run detail data and UI so completed runs can be inspected.
3. Add runtime triggers for diff and validation capture.
4. Add the review surface that combines final response, diff, validation, and next action.
5. Install Visual Studio Build Tools with the Visual C++ linker and verify the Tauri build path.

## Later Roadmap

### Rich Codex Control

After the `codex exec --json` loop works, add SDK/app-server support behind the same runtime
boundary for interactive sessions, approvals, steering, interrupts, and live turn state.

### Workflow Engine

Add editable workflows for branch naming, worktree strategy, Codex profile/sandbox settings,
preflight commands, post-run validation, attention-state transitions, and cleanup policies.

### Review And Delivery Tools

Add stronger diff summaries, validation history, commit helpers, PR preparation, and risk/next-action
summaries.

### Conversation Import

Support official ChatGPT export import for archive/search/linking. This should remain an import and
organization feature, not live ChatGPT control.

### Notifications And Automation

Add local notifications, snooze/review scheduling, optional local web views, and scheduled workflows.

### Hardening And Recovery

Add process supervision, startup reconciliation, stale worktree/session recovery, error boundaries,
database backup/export, and security review.

## References To Check When Needed

Use current official Codex docs before implementation that depends on Codex behavior:

- Codex non-interactive mode
- Codex CLI reference
- Codex SDK
- Codex app-server
- Codex authentication and configuration
