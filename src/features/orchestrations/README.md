# Orchestrations Feature

`src/features/orchestrations` owns the user-facing orchestration workflow:
registry overview, intake draft creation, build package review, orchestration
workspace navigation, blocker review, planner/slice inspection, and runtime
status presentation.

The reusable orchestration client contract and state/domain helpers live outside
the feature in `src/application` and `src/domain`. This feature should consume
those contracts through injected clients.

Current state: the page is a first-stage extraction from the old app monolith.
When changing it next, split controller state, projection helpers, and rendering
into separate folders instead of growing the page file.
