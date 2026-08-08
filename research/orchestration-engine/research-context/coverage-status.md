# Research coverage and evidence gaps

This is a snapshot of what the repository already explains and where its evidence remains thinner. It is not a work plan or a final architecture, disposition or completeness assessment.

| Subject | Current coverage | Main artifacts | Known gap or useful depth |
| --- | --- | --- | --- |
| product capability scope | broad capability groups, reachability and branch-local distinctions | `catalogs/capability-landscape.md`; role readings | disposition criteria and value evidence per capability |
| frontend experiences | product entry, navigation, major views, reuse seams and non-product routes | `catalogs/frontend-experience-map.md`; designer reading | rendered-state inventory, interaction audit and responsive evidence |
| Rust backend and Tauri | crate/composition boundary, major modules, events and command surface | `catalogs/backend-and-tauri.md`; `catalogs/tauri-operations.md` | dependency graph, ownership pressure and command-by-command consumer verification |
| durable state | product databases, schema families, non-SQL artifacts and compatibility stores | `catalogs/durable-state.md` | table-level lifecycle/retention ownership and migration necessity |
| MCP | server variants, endpoints, tools, transport, security and authority splits | `catalogs/mcp-servers-and-tools.md` | live lifecycle evidence per variant and convergence design |
| Harness and configuration | static catalog, generated profiles, durable revisions, prompts, skills and policies | `catalogs/harness-and-configuration.md` | precedence rules, effective configuration resolution and edit-authority design |
| CLI/process integration | Codex, Git, helper processes, environment inheritance and operator tools | `catalogs/cli-process-and-environment.md` | platform/runtime failure modes and explicit environment policy |
| representative behavior | six end-to-end traces plus three evidence-selected deep traversals | `operation-traces/`; `evidence-passes/` | exception paths and multi-session concurrency where they change the product explanation |
| behavior-led evidence | ten stable-snapshot passes, three moving-state passes and four independent signal ledgers | `evidence-passes/`; `discovery-sweeps/` | additional journeys only where they test or overturn recurring structures |
| historical evolution | major branch lines, chronology and decision records | `history/` | conversation-derived intent, abandoned alternatives and integration reconciliation |
| role interpretation | owner, architect, developer and designer readings | `perspectives/` | cross-role tensions and decision-ready options |

## Stronger than a directory survey

The current repository links user-facing concepts to concrete frontend, Rust, persistence, process and MCP artifacts. It also distinguishes checked-out behavior from sibling-line functionality and separates code presence from reachability, live proof and acceptance.

## Not established yet

The first pass does not yet establish:

- a final productive/test/legacy disposition for every file;
- dead-code status based only on lack of an obvious caller;
- which sibling branch should become the integration authority;
- live native-provider, packaged-application or retained-data behavior;
- the value, cost or risk ranking needed for keep, tune, prune, segment, centralize or refactor decisions;
- a complete visual design-system or interaction-state inventory;
- conversation-history intent where it is not already captured in commits and repository records;
- exhaustive semantic review of every uncommitted or detached worktree; the register currently prioritizes materially different source states.

Those are deliberate next-depth investigations. The catalog structure provides stable places to attach their evidence without forcing early conclusions.
