# Active product database

`codex-orchestrator-active-v3.sqlite` is one physical database with separately owned product capabilities.

## Data families and owners

| Data family                      | Capability owners                                                 |
| -------------------------------- | ----------------------------------------------------------------- |
| Application configuration        | `native_profiles`                                                 |
| Operational control              | `orchestration::execution_support`                                |
| Authored definitions             | `workflows` definitions and Orchestration-owned Harness revisions |
| Runtime transactions             | Workflow runtime, `agent_sessions`, and `harness_engine` bindings |
| Product-development meta-process | `product_decisions` and Epic, Sprint, and Work Unit orchestration |

These families classify data; they do not share domain authority. Each capability owns its schema and repository contracts.

## Access norm

Product composition creates one `ActiveDatabase` handle per process. Persistence adapters use query-only reader connections and one managed immediate-transaction writer. SQLite WAL and the busy timeout coordinate separate desktop and CLI processes.

Application services do not receive connections or transactions. Managed operations cannot be nested inside a managed write, and domain errors explicitly roll that write back. A managed write contains SQL only; filesystem, Git, MCP, provider, and notification work happens outside it. Cross-capability operations remain staged application operations rather than one database transaction.

Separate legacy, Worktree Runtime, Worktree Review, and test databases do not use this active-product boundary.
