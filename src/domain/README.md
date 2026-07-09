# Domain

`src/domain` owns business records, policies, planning logic, projections, and
store contracts for the orchestrator model. Domain code should be deterministic
and independent from UI and runtime details.

Rules:

- Depend only on domain-local types and pure helpers.
- Keep React, Tauri, SQLite, Git command execution, filesystem access, and Codex
  runtime calls out of this layer.
- Model orchestration concepts and invariants here before application use cases
  expose them to the UI.
- Define contracts in domain terms when persistence or runtime implementations
  need to satisfy domain behavior.

Any remaining imports that blur domain, application, and infrastructure ownership
should be checked and resolved later. This README does not perform that cleanup.
