# Expert-developer perspective

## Codebase shape

The productive implementation is split between:

- a React/TypeScript product and projection layer;
- one Rust crate containing Tauri transport, application services, persistence, MCP hosting and process adapters;
- repository-owned skills, JSON configuration and operator tools;
- extensive deterministic tests and recorded development surfaces.

The cleanest implementation model is Agent Sessions. The greatest implementation density lies in orchestration native-query/presentation code, Sprint/bootstrap transitions, Native Profiles and Harness Management.

## High-value implementation components

- `agent_sessions/`: reusable provider-neutral Session lifecycle with durable evidence.
- `runtime/codex/` and `runtime/processes/`: focused CLI adapter and process supervision.
- `work_unit_execution_harness.rs` plus execution/Git authority modules: bounded capability construction.
- native-query decoder and product composer: strict frontend boundary over native state.
- `AgentSessionWorkspace`, `ConversationViewport`, `SharedAgentSessionPanel` and transcript projection: reusable conversation UI.
- `DetailWorkspace`, `ResizableSplitSurface`, `ProductViewHeader`, Markdown/editor components: emerging UI toolkit.
- deterministic recorded surfaces and debug review runtime: strong verification infrastructure when classified correctly.

## Hotspots and mixed responsibilities

### Rust

- `bootstrap_transition.rs`: over 10,000 lines, although tests begin around line 2,039;
- `native_profiles.rs`: over 8,000 lines;
- legacy `lib.rs`: over 6,000 lines;
- `orchestration/repository.rs`: nearly 4,900 lines plus a large separate test file;
- `sprint_runner_transition.rs`: over 4,600 lines;
- `orchestration/application.rs`, `execution_support.rs`, `storage.rs` and `mcp.rs` remain substantial.

The transition and Native Profile hotspots mix several change reasons. Raw size is not itself the problem; coupled state-machine, SQL, transport, MCP, prompt and external-effect changes are.

### TypeScript/React

- `nativeQuery.ts`: over 3,200 lines;
- `ConversationHarnessInspector.tsx`: over 2,100 lines;
- recorded orchestration input: roughly 1,450 lines;
- product read-model composer: roughly 1,360 lines;
- `SprintWorkspace.tsx`: over 1,000 lines;
- `WorkUnitDetailWorkspace.tsx`: roughly 860 lines.

The projection pipeline is strict and valuable, but native schema, decoder, composer and UI can require synchronized large edits.

## Productive code versus retained seams

| Category | Examples |
| --- | --- |
| productive | Agent Sessions, Plan Builder, bootstrap, Sprint/Work Unit transitions, MCP tools, native query, accepted integration |
| productive but incomplete wiring | Native Profile launch-policy convergence, native MCP visible action, contextual File Review producer, generic Harness inspection |
| debug/operator | Rust Worktree Review, app inspector, proof examples, runtime status server |
| deterministic development | recorded orchestration corpus, File Review fixtures, Agent Session scenario harness |
| compatibility | recorded workflow geometry and compatibility projections |
| quarantined | legacy Task Dashboard/backend/Node local runtime |
| branch-only | Product Decisions and final settlement |

Compile inclusion should not be used as a proxy for these categories.

## Ownership friction

- Agent Sessions import a split-surface component from the orchestration feature.
- global Markdown editing/display depends on Agent Session-owned wrappers.
- File Review and Harness Management also reach into Agent Session presentation code.
- Harness tool/prompt configuration is repeated across JSON, Rust, skills and endpoint descriptions.
- Node and Rust Worktree Runtime implementations overlap.
- npm and pnpm workspace/lock artifacts coexist while scripts/docs use npm.
- global CSS still ships a large legacy task/dashboard section.

These are signs that reusable capability emerged through feature work before stable package ownership was established.

## Correctness patterns worth preserving

- exact idempotent application invocation IDs;
- compare-and-swap Git refs and target-current versions;
- durable intent before external effect, followed by restart reconciliation;
- revalidation of pinned Harness revision/digest/reference;
- separate launch acceptance, lifecycle, semantic completion, evidence and settlement;
- bounded opaque references instead of raw agent-provided paths;
- structured attention rather than guessed recovery.

## Testing and evidence observations

- The repository has a high test density; frontend `src/` sampling found 117 test files among 335 files.
- Large Rust modules contain extensive inline proof suites, so source-line size exaggerates release implementation size while still increasing navigation cost.
- Recorded UI sources are both valuable acceptance fixtures and a source of compatibility drift.
- Live/ignored tests are evidence seams, not automatically product paths.
- Stale comments and docs already contradict mounted or implemented behavior, so code-and-reachability evidence must lead documentation cleanup.

## Candidate engineering investigations

- generate a machine-readable operation/tool/reachability inventory from registrations and adapters;
- measure production/test lines separately for hotspots;
- identify common reconciliation protocols before extracting abstractions;
- compare Node/Rust Worktree Runtime parity before removing either;
- determine whether legacy code can be compiled as a separate compatibility crate/package;
- introduce explicit shared-UI ownership rather than cross-feature imports;
- establish one package manager and one build-entry policy;
- expose actual pinned Harness data to inspection rather than duplicating it in TypeScript.

## Questions to carry forward

- Which tests depend on legacy or recorded compatibility artifacts in ways that block extraction?
- Which migrations protect real user data versus synthetic predecessor fixtures?
- Where do multiple SQLite connections contend under realistic orchestration load?
- Which process launchers should share environment hardening?
- Can native-query contracts be generated or decomposed without weakening strict validation?
- What branch integration sequence avoids creating a synthetic tree that has never been validated?
