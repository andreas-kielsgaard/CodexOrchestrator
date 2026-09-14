# Rust/Tauri boundaries

## Active architecture

New product work belongs in focused modules under:

- `src/agent_sessions/` for Agent Session domain, application, persistence, and transport;
- `src/runtime/` for provider adapters and supervised processes;
- `src/storage.rs` for active database composition;
- `src/active_app.rs` for Tauri composition.

Create a focused module and port when a new capability does not fit these areas.

## Crate entry point

Keep `src/lib.rs` limited to module declarations and application entry points.

## Compilation

From the repository root, use `npm run check:rust`, `npm run test:rust:fast`, or `npm run build` (complete runnable application; launch separately). These commands accept `-- --cache=auto|local|shared`. Auto prefers local profile artifacts, otherwise shared sccache when available. Local preserves incremental compilation; shared disables it for that invocation. Use `npm run build -- --debug` for application debugging and `npm run build:frontend` for frontend-only compilation. Keep useful caches; see `docs/development.md` for explicit clearing.
