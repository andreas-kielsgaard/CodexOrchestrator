# Open Tasks Feature

`src/features/openTasks` owns the Open Tasks workflow: dashboard loading, repo
onboarding, task composition, task editing, run launching, task state changes,
and task run detail inspection.

Folder roles:

- `controllers`: feature UI state and workflow coordination.
- `viewModels`: pure Open Tasks projection and form shaping.
- `views`: Open Tasks rendering components and view compositions.

Open Tasks controllers consume capabilities and application contracts. They
should not call Tauri, SQLite, Git, Codex, or filesystem adapters directly.
