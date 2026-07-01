# Architecture Notes

Date: 2026-07-01

## Stack Choice

This bootstrap follows the roadmap stack:

- Tauri v2 for the desktop shell.
- React, TypeScript, and Vite for the frontend.
- Rust commands as the backend boundary for local filesystem, process, Git, and future Codex-adapter work.

The current Rust side exposes only an `app_metadata` command. It exists to prove the command boundary without starting Codex integration, Git scanning, or persistence prematurely.

## Local-First Boundary

The UI treats tasks as the unit of attention. Technical anchors such as repos, branches, worktrees, conversations, and artifacts are reserved for later slices. Frontend code lives under `src/`, while local runtime commands live under `src-tauri/`.

Future slices should keep filesystem/process-heavy behavior behind Tauri commands or adapter modules instead of calling Git, SQLite, or Codex directly from React components.

## Tooling Note

Rust/Cargo were not available on the worker PATH during bootstrap, so desktop verification could not run in this environment. Frontend build, lint, formatting, and tests remain independently verifiable through npm scripts.
