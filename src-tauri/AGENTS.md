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
