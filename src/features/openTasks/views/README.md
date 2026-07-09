# Open Tasks Views

Open Tasks views render the feature's screen, forms, review layout, task cards,
run controls, and task detail panel.

Views should receive view-model data and callbacks from controllers. They should
not own application calls, persistence, runtime orchestration, or cross-feature
state.

If a view starts accumulating feature workflow state or projection helpers, split
that responsibility into `controllers` or `viewModels`.
