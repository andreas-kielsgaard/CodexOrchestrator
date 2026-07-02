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

Important gaps:

- No UI/runtime composition that chooses the app database path and exposes stores to application
  services.
- No run composition service that connects Codex execution to lifecycle, stores, artifacts, and
  events.
- No persisted dashboard UI; the visible app is still seed/demo driven.
- No repo registration/worktree creation UI path.
- No task/run detail view.
- No diff collector or validation command runner.
- `npm run build:tauri` is blocked until Rust/Cargo are installed or on `PATH`.

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

1. Compose runtime adapter, JSONL parser, lifecycle recorder, stores, artifacts, and events.
2. Add repo registration/scanning and worktree selection/creation.
3. Replace seed dashboard behavior with persisted task CRUD.
4. Add run controls and task/run detail UI.
5. Add diff capture and validation command execution.
6. Add the review surface that combines final response, diff, validation, and next action.

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
