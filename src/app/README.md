# App

`src/app` owns the React application root and UI-serving composition. It wires the
desktop shell into controllers, view models, and views without taking ownership of
domain policy or concrete infrastructure.

Rules:

- The app root may receive runtime dependencies through props or composition
  helpers.
- Controllers own UI state and translate events into application capabilities.
- View models and presenters shape display data without side effects.
- Views render props and raise callbacks; they do not call persistence, Git,
  Codex, SQLite, or Tauri adapters directly.
- App code may depend on `src/application` contracts and app-local helpers, but
  should not depend on concrete infrastructure implementations.

Redundant state or duplicate helpers left during the app split should be checked
and resolved in a later cleanup pass.
