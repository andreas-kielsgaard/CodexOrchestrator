# Commands

`src/application/commands` is the canonical home for state-changing application
contracts and use cases.

Use this folder for operations that create, update, start, archive, register,
scan, validate, or otherwise change application state.

Rules:

- Depend on domain modules and application ports, not React or concrete
  infrastructure adapters.
- Keep UI workflow state in feature/app controllers, not in command modules.
- Keep rendering labels and feature display shaping in view models or
  presenters.

Some files currently re-export root-level application modules so product work on
`main` can move into the new architecture without losing agent-session and
orchestration surfaces. The root modules can become compatibility shims after
callers migrate.
