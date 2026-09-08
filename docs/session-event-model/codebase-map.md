# Codebase map

This map distinguishes elements that appear useful to reuse, extract, preserve as a pattern or
retire. Confirm an element's current consumers before changing it; a targeted implementation may
discover a better extraction boundary than the one suggested here.

## Functional elements

| Existing element                                               | Disposition                    | Target use                                                                                      |
| -------------------------------------------------------------- | ------------------------------ | ----------------------------------------------------------------------------------------------- |
| `src-tauri/src/execution_configuration/`                       | Reuse and complete             | Canonical Runtime, Capability, Node and Session Profile vocabulary and resolution.              |
| `src-tauri/src/session_events/`                                | Reuse and complete             | Generic definition, materialization, addressing, dispatch and delivery record kernel.           |
| `src-tauri/src/workflows/compiler.rs`                          | Reuse and extend               | Compile authoring state into Session Event definitions.                                         |
| `src-tauri/src/workflows/address_references.rs`                | Reuse                          | Translate Workflow instance, node and connection identities into generic references.            |
| `src-tauri/src/agent_sessions/session_event_adapter.rs`        | Reuse as adapter               | Implement Session Event directory and invocation ports.                                         |
| Agent Session lifecycle, runtime, transcript and repository    | Preserve                       | Sessions remain the durable runtime concept. Replace configuration derivation and presentation. |
| Identity domain and clients                                    | Preserve                       | Keep identity independent from operational configuration.                                       |
| Workflow draft, save and activation coordination               | Preserve the lifecycle pattern | Replace legacy configuration payloads while retaining explicit authoring state.                 |
| Tauri client -> application contract -> infrastructure adapter | Preserve the pattern           | Apply to new Capability, Session Profile and Session Event surfaces.                            |
| Available/unavailable reads with reasons                       | Preserve the pattern           | Represent provider-managed or unsupported capability controls truthfully.                       |

## UI elements

| Existing element                                      | Disposition                        | Target use                                                     |
| ----------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------- |
| `src/components/CollapsibleSection.tsx`               | Reuse                              | Standard disclosure section across new editors and inspectors. |
| `src/components/MarkdownContent.tsx`                  | Reuse                              | Prompt rendering and composed-prompt inspection.               |
| `src/components/MarkdownEditor.tsx`                   | Reuse after cleanup                | Remove its Agent Session-specific rendering dependency.        |
| `src/features/identities/`                            | Reuse and consolidate              | Identity selection and color/shape/initial presentation.       |
| `ConversationViewport`, `AgentSessionWorkspace`       | Reuse                              | Session composition and display boundary.                      |
| `transcriptProjector.ts`                              | Reuse and extend                   | Add event source and provenance presentation.                  |
| `workflowNodeDrag.ts`                                 | Reuse                              | Pure canvas drag geometry.                                     |
| `workflowEditorController.ts`                         | Reuse after replacing config types | Retain port-driven draft/save behavior.                        |
| Searchable selectors in `HarnessDefinitionEditor.tsx` | Extract                            | Generic catalogue single- and multi-select controls.           |
| Harness field, card and dialog presentation           | Extract selectively                | Generic editor shell, field and state presentation only.       |
| Workflow connection activity popup                    | Recast                             | Event group and delivery projection.                           |

Duplicate identity badge and marker components should converge under `features/identities` or a
small generic presentation package.

## Replacement surfaces now present

- Capability Profile repository, named/revisioned service and Runtime Profile query.
- immutable Session Profile resolution, persistence, query and digest verification.
- Workflow authoring persistence with embedded Node Profiles, node-state copy and activation-time
  compilation.
- Session Event materialization, addressing, dispatch, SQLite records and read queries.
- coherent Agent Session/profile/address creation and optional Agent Identity snapshot assignment.
- invocation-local direct-user model/reasoning selection.
- browser-safe application contracts and Tauri clients under the target frontend buckets.
- controlled, unmounted Capability, Node and Session Profile UI components.
- controlled, unmounted trigger, prompt-source, target, event-group and delivery components.
- reusable catalogue, resolved-value and validation controls plus per-message runtime controls.

## Remaining integration surfaces

- mount the new clients and components in Workflow and Agent Session application composition;
- adapt Workflow canvas/controller state to the replacement authoring contract;
- add the first concrete Workflow-owned referenced-content resolver for expected-file use;
- connect the currently required application/MCP trigger ingresses to compiled definitions;
- project Session Event provenance into transcript messages;
- add a compact compiled-event summary and navigation from deliveries to their sources;
- retire legacy Harness, Role and Session model-override paths after the replacement UI is usable.

The existing replacement components are a starting point rather than a constraint on the eventual
visual composition. Agents implementing a specific surface should reuse a component when its
contract fits and reshape it when more local product evidence supports a clearer boundary.

## Retire after replacement

- `HarnessEditor` and `HarnessDefinitionEditor` as domain compositions.
- `HarnessEffectiveConfiguration` and equivalent mixed aggregates.
- generic Workflow Role definitions, catalogues, inheritance and overrides.
- Session-owned Harness drafts, revisions and update commands.
- Session-level model override persistence.
- hook, discovery, timing and update-policy concepts without current functional guarantees.
- legacy Workflow node-configuration adapters.
- recorded Harness development sources after the replacement UI has suitable fixtures.

## Target frontend buckets

```text
src/components/                 generic disclosure, form, markdown and dialog primitives
src/application/
  executionConfiguration/      browser-safe profile reads and commands
  sessionEvents/               definitions, occurrences and read projections
  workflows/                   authoring and compile validation
  agentSessions/               Session reads and invocation
  identities/                  identity reads and assignment
src/features/
  executionConfiguration/      Capability, Node and Session Profile UI
  sessionEvents/               trigger, prompt, target and delivery UI
  workflows/                   canvas and feature composition
  agentSessions/               workspace, transcript and composer
  identities/                  identity presentation and selection
src/infrastructure/             Tauri clients and external adapters by domain
src/app/                        routing and application composition only
```

Physical moves may be incremental. Establish public consumption surfaces before moving stable
files solely for visual symmetry.
