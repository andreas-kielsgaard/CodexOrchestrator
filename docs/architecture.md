# Architecture Notes

Date: 2026-07-01

## Stack Choice

This bootstrap follows the roadmap stack:

- Tauri v2 for the desktop shell.
- React, TypeScript, and Vite for the frontend.
- Rust commands as the backend boundary for local filesystem, process, Git, and future Codex-adapter work.

The current Rust side exposes only an `app_metadata` command. It exists to prove the command boundary without starting Codex integration, Git scanning, or persistence prematurely.

## Local-First Boundary

The UI treats tasks as the unit of attention. Technical anchors such as repos, branches, worktrees, conversations, validation runs, events, and artifacts are modeled separately so project/repo drilldowns can explain where work lives without becoming the dashboard's primary organizing shape.

Future slices should keep filesystem/process-heavy behavior behind Tauri commands or adapter modules instead of calling Git, SQLite, or Codex directly from React components.

## Domain Boundary

The TypeScript domain layer lives under `src/domain/`:

- `model.ts` defines the core product records: Project, Repo, Branch, Worktree, Conversation, Task, TaskRun, Artifact, ValidationRun, and Event.
- `Task.executionState` tracks what the work is doing, while `Task.attentionState` tracks what kind of human attention it needs.
- `dashboardProjection.ts` derives the Open Tasks dashboard groups from domain records. React components should consume this projection instead of owning grouping rules directly.
- `seedData.ts` provides demo records until SQLite persistence and repository APIs are introduced.

SQLite migrations, Rust database commands, Git scanning, and Codex runtime integration are intentionally outside this slice.

## Git Adapter Boundary

Git output parsing lives under `src/infrastructure/git/` as pure TypeScript infrastructure code. The current adapter foundation normalizes parseable command output from `git status --porcelain=v1 -z`, `git branch --format=...`, and `git worktree list --porcelain -z` into scan facts that can later feed the domain `Repo`, `Branch`, and `Worktree` records.

Repo scan assembly is also pure TypeScript: raw command outputs, a root path, optional default branch, and scan timestamp are composed into `GitRepoScanResult` without executing Git. A thin domain-facing mapping layer derives normalized repo, branch, and worktree facts from that scan while keeping non-root worktree dirtiness explicit as `unknown` until each worktree can be scanned directly.

React components should not invoke Git or parse Git output directly. Future Tauri/Rust or sidecar command execution should enter through the thin Git adapter boundary and hand raw command output to these parsers.

## Tooling Note

Rust/Cargo were not available on the worker PATH during bootstrap, so desktop verification could not run in this environment. Frontend build, lint, formatting, and tests remain independently verifiable through npm scripts.
