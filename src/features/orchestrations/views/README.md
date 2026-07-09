# Orchestration Views

Orchestration views render the registry, add-orchestration flow, build package
details, workspace tabs, plan map, blocker notices, planner/slice views, and
conversation surfaces.

The current page still contains controller state and projection helpers from its
initial extraction. Treat that as migration debt: when touching a sub-flow, move
state into a controller and pure shaping into a view-model or presenter before
adding more responsibilities.
