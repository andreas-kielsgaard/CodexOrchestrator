# Queries

`src/application/queries` is the canonical home for read-only application
contracts and use cases.

Use this folder for operations that load snapshots, detail views, runtime status,
or other application data without mutating state.

Queries may depend on domain store interfaces and application ports. They should
not import React components, feature controllers, app-shell modules, or concrete
infrastructure adapters.
