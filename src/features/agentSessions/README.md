# Agent Sessions Feature

`src/features/agentSessions` owns the user-facing Agent Session workflow.

The technical functionality for launching, routing, formatting, and storing
agent sessions lives in `src/application`; this feature should consume that
functionality rather than reimplement it.

Current state: the page is a first-stage extraction from the old app monolith.
When changing it next, prefer splitting feature state into `controllers`, pure
display shaping into `viewModels`, and rendering into `views`.
