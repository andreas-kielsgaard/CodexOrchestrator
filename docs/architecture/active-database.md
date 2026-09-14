# Active product database

`codex-orchestrator-active-v3.sqlite` is one physical database with separately owned product capabilities.

## Data families and owners

| Data family                      | Capability owners                                                               |
| -------------------------------- | ------------------------------------------------------------------------------- |
| Application configuration        | `native_profiles` and `repository_catalog`                                      |
| Operational control              | `orchestration::execution_support`                                              |
| Authored definitions             | Workflow recipes, Capability Profiles and Orchestration-owned Harness revisions |
| Runtime transactions             | Session Events, recipe instances, `agent_sessions`, and mediated MCP bindings   |
| Product-development meta-process | `product_decisions` and Epic, Sprint, and Work Unit orchestration               |

These families classify data; they do not share domain authority. Each capability owns its schema and repository contracts.

## Access norm

Product composition creates one `ActiveDatabase` handle per process. Persistence adapters use query-only reader connections and one managed immediate-transaction writer. SQLite WAL and the busy timeout coordinate separate desktop and CLI processes.

Application services do not receive connections or transactions. Managed operations cannot be nested inside a managed write, and domain errors explicitly roll that write back. A managed write contains SQL only; filesystem, Git, MCP, provider, and notification work happens outside it. Cross-capability operations remain staged application operations rather than one database transaction.

The repository catalog stores only durable local registrations and their disclosure provenance.
Branch and worktree inventories are live Git observations and are not copied into ActiveDatabase.
Codex and GitHub discovery also remain external observations. Their filesystem, Git, and provider
processes complete before a catalog write begins.

Older database files left untouched by startup, Worktree Review operational databases, and test databases do not use this active-product
boundary. Worktree Review's separate database owns selected review context, source and worktree
evidence, builds, attempts, outputs, retention, cleanup, and receipts; it does not own the shared
repository catalog.

## Source and rationale

`src-tauri/src/product_database/mod.rs` assembles capability-owned schemas; `active_schema.rs` and `storage.rs` identify the active schema. `src-tauri/src/persistence/active_database.rs` owns managed reads and writes. Shared storage does not create shared domain authority: a repository operation is not permission to perform a provider or Git effect inside a database transaction.

The managed access decision followed a real concurrent `database is locked` failure and the user's requirement that parallel work remain possible. The discussion chose a small managed operator boundary rather than exposing connections to application code. Source: task `01a03977-7ea1-7aa0-9db4-1728cb02dd06`, raw lines 6540, 6678, 6700, 6777, 6905 and 6983; demonstrated-scope acceptance at 9790. See [validation evidence](../validation-evidence.md#database-and-repository-integration) for the proof boundary.

## Prototype history and retained data

The July prototype used `codex-orchestrator.sqlite` and a different migration ledger. Its implementation and prototype-table quarantine helper were retired at `ac18781`; current `storage.rs` opens the active-v3 database and leaves older files untouched. Its reserved entries were `006_orchestration_drafts_schema` at position 5, `007_orchestration_stage_runs_schema` at 6, and `008_agent_sessions_schema` at 7. These are historical identities, not the numbering scheme for the current active-v3 schema. The former instruction to start forward work at migration 009/position 8 belonged to that ledger.

A source-code reset did not reset a developer's SQLite files. The enduring lesson is to preserve the complete stopped application's data, including relevant WAL/SHM sidecars, and exercise clean-install and retained-data migrations on disposable copies. Retained records require an explicit forward transformation; rewriting migration history or silently treating archived tables as current data would lose that distinction. The old reset recipe is preserved at `e2bfc6c:docs/agent-session/prototype-database.md`; it is not a current startup procedure.

Return to [architecture](../architecture.md) for composition and filesystem ownership.
