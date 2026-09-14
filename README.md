# Codex Orchestrator

Codex Orchestrator is a local-first desktop surface for doing work with agents. Its current core is
a durable Agent Session: a text interaction context that can stream Codex CLI work, preserve the
technical record, reopen after an app restart, and continue the same external Codex thread. The
older task/dashboard implementation remains in the repository for migration compatibility and
isolated tests, but it is quarantined from the mounted UI and its Tauri commands fail closed.

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

The launcher prepares the local Cargo/MSVC/Codex paths and starts `npm run dev:tauri`. The app shows
a loading screen until the Tauri command backend responds.

## Scripts

- `npm run dev`: start the React/Vite app.
- `npm run dev:status`: explicitly start the optional runtime status server for the review CLI.
- `npm run dev:tauri`: start the Tauri desktop shell.
- `npm run mark:stale -- --target backend --reason "Rust command changed"`: record a stale-state
  marker for review tooling.
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
  application/         Browser-safe client contracts
  domain/              Domain types, seed records, and dashboard projection
  features/            Mounted Agent Session screen and quarantined legacy task components
  infrastructure/      Tauri command adapters
  test/                Test setup
src-tauri/
  src/agent_sessions/  Durable domain, repository, lifecycle, and transport
  src/runtime/         Codex adapter and operating-system process supervisor
docs/
  architecture.md      Stack and boundary notes
  agent-session/       Agent Session decisions, implementation evidence, and execution ledger
  task-logs/           Worker completion logs
```

## Agent Session Architecture

The implemented vertical slice and its deliberate boundaries are documented in
[`docs/agent-session/README.md`](docs/agent-session/README.md). The earlier integrated prototype
remains on archive branches only as evidence; it was not merged wholesale.

## Optional Review Runtime Status

The review CLI can inspect a separately started runtime status server. Start it with
`npm run dev:status` when needed; `launch-dev.bat` does not start it. Its state is stored in the
Git-ignored `.dev/runtime-status.json`. The current application has no stale-state banner.

To record a marker for review tooling, call:

```bash
npm run mark:stale -- --target frontend
npm run mark:stale -- --target backend --reason "Tauri command changed"
```

Valid targets are `app`, `frontend`, and `backend`.
