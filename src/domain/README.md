# Domain

`src/domain` owns pure business concepts: records, domain services, projections,
state machines, validation rules, and store interfaces expressed in domain
language.

Domain code may depend on other domain modules. It should not depend on
`src/application`, `src/features`, `src/app`, `src/infrastructure`, React,
Tauri, SQLite, Git, Codex, or filesystem adapters.

If a rule describes what the product means, put it here. If a module coordinates
external systems or user-interface state, it belongs elsewhere.
