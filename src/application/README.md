# Application

`src/application` owns use cases and contracts that coordinate domain behavior for
the UI and runtime boundaries. It is the layer that turns user intent into domain
operations and returns stable results to app-facing callers.

Rules:

- Commands perform state-changing use cases.
- Queries load read-only use-case data.
- Ports describe technical capabilities needed by use cases.
- Presenters and validation helpers may shape application-level output without
  depending on React.
- Depend on `src/domain` and application-local ports; do not depend on `src/app`
  or concrete infrastructure adapters.

Broad clients and overlapping use-case helpers should be checked for redundancy in
a later cleanup pass. Do not collapse them as part of contract documentation.
