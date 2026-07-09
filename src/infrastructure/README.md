# Infrastructure

`src/infrastructure` owns concrete adapters for external systems and local
runtime details.

Examples include Tauri command clients, browser development clients, Codex
runtime integrations, Git adapters, SQLite stores, validation process runners,
clipboard helpers, and local runtime composition.

Infrastructure may implement application ports and application/domain store
contracts. It should not own React workflow state, feature view models, domain
policy, or app-shell routing decisions.

When adding an adapter, keep the side-effecting code here and expose it through a
capability, application contract, or application port.
