# Rust/Tauri boundaries

## Active architecture

New product work belongs in focused modules under:

- `src/agent_sessions/` for Agent Session domain, application, persistence, and transport;
- `src/runtime/` for provider adapters and supervised processes;
- `src/storage.rs` for active database composition;
- `src/active_app.rs` for Tauri composition.

Create a focused module and port when a new capability does not fit these areas.

## Legacy quarantine

Most of `src/lib.rs` is the archived task/run implementation. It remains compiled for migration
compatibility and isolated tests. Its registered task commands must remain fail-closed.

Do not:

- build new features from its task, run, conversation, Git, validation, process, DTO, or database
  implementation;
- call its legacy helpers or handlers from active modules;
- add new product behavior, migrations, commands, or tests to the legacy implementation;
- move active logic into `lib.rs` for convenience.

If active work needs a capability found only in `lib.rs`, define the current requirement first.
Then extract and disentangle the minimum capability into a focused active module, or rewrite it
behind a new port. Do not preserve legacy coupling merely to reuse code.

Allowed legacy edits are fail-closed compatibility maintenance, extraction, deletion, and tests
that prove quarantine or safe migration behavior. Keep the crate entry point and module declarations
in `lib.rs` minimal when touching them.
