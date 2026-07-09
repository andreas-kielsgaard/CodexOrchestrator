# Application

`src/application` owns runtime-independent application functionality: use cases,
service contracts, app-facing clients, technical ports, presenters, and
coordination that is not React-specific.

Application modules may depend on `src/domain` and application-local ports. They
should not import React components, feature controllers, app-shell modules, or
concrete infrastructure adapters.

Current subfolders:

- `commands`: state-changing application use cases and command-facing
  contracts. Some files are compatibility adapters while migration is in
  progress.
- `queries`: read-only application query contracts and query use cases.
- `ports`: interfaces implemented by infrastructure adapters and consumed by
  application use cases.
- `presenters`: pure application-level output shaping that is reusable outside
  React.

Root-level files remain for existing import paths and broad application services
such as agent sessions and orchestration. Prefer canonical subfolders for new
command/query/port/presenter work.
