# Codex Orchestrator

Codex Orchestrator is a local-first desktop control plane for Codex-driven work. It uses Tauri v2,
React, TypeScript, Vite, SQLite, and local runtime adapters to track tasks, repos, worktrees, Codex
runs, artifacts, validation results, and review state.

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

For the desktop development loop on Windows, run the launcher from the repo root:

```bat
launch-dev.bat
```

The launcher prepares the local Cargo/MSVC/Codex paths, starts the runtime status server, clears any
old stale marker, and then starts `npm run dev:tauri`. The app shows a loading screen until the
Tauri command backend responds.

To load a project, add its Git repository root in the app. For this repo, use:

```text
C:\Users\user\Documents\Code Projects\Codex Orchestrator
```

The app scans Git branches and worktrees from the repo and persists the runnable worktree anchors.
You can also scan a designated folder, such as `C:\Users\user\Documents`, to find candidate repos
without scanning the whole disk.

## Scripts

- `npm run dev`: start the React/Vite app.
- `npm run dev:status`: start the local runtime status server used by the stale-state banner.
- `npm run dev:tauri`: start the Tauri desktop shell.
- `npm run mark:stale -- --target backend --reason "Rust command changed"`: mark app,
  frontend, or backend state stale so the running UI offers a refresh.
- `npm run clear:stale`: clear the runtime stale marker.
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

## Runtime Status

The runtime status server stores its local state in `.dev/runtime-status.json`. It is intentionally
ignored by Git. When a running dev app should refresh after code changes, call:

```bash
npm run mark:stale -- --target frontend
npm run mark:stale -- --target backend --reason "Tauri command changed"
```

Valid targets are `app`, `frontend`, and `backend`.
