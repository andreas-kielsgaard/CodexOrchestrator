# Codex Orchestrator

Codex Orchestrator is a local-first desktop control plane for Codex-driven work. The current slice establishes a Tauri v2 + React + TypeScript + Vite skeleton plus an attention-first Open Tasks dashboard driven by TypeScript domain records and projection logic.

## Prerequisites

- Node.js 24+
- npm 11+
- Rust and Cargo for Tauri desktop commands and packaging

Rust is required for `npm run dev:tauri` and `npm run build:tauri`. The frontend-only app can run with Node/npm.

## Setup

```bash
npm install
npm run dev
```

The Vite dev server runs on `http://localhost:1420`.

## Scripts

- `npm run dev`: start the React/Vite app.
- `npm run dev:tauri`: start the Tauri desktop shell.
- `npm run build`: type-check and build the frontend.
- `npm run build:tauri`: build the desktop app.
- `npm run lint`: run ESLint.
- `npm run format:check`: check Prettier formatting.
- `npm run test`: run Vitest.

## Project Layout

```text
src/
  app/                 React application shell
  domain/              Domain types, seed records, and dashboard projection
  infrastructure/      Tauri command adapters
  test/                Test setup
src-tauri/             Tauri v2 Rust shell and command boundary
docs/
  architecture.md      Stack and boundary notes
  task-logs/           Worker completion logs
```

## Current Scope

The app currently shows demo task records projected into:

- Needs action now
- Review / decide
- Working
- Waiting
- Later

The domain model includes projects, repos, branches, worktrees, conversations, tasks, task runs, artifacts, validation runs, and events. Execution state and attention state are separate fields on tasks so a completed task can still need review, and a running task can be tracked as waiting on an agent.

Codex protocol integration, Git scanning, SQLite persistence, and workflow execution are intentionally left for follow-up slices.
