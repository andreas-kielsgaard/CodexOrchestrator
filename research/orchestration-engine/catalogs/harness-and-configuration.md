# Conversation Harness and configuration authority

## Why this is both functionality and configuration

A Conversation Harness determines what kind of agent is launched, what it is told, which skills it may discover, which MCP tools it receives, where it works, and which runtime policies apply. Those choices are executable product behavior. They are also represented as configuration that can be inspected, versioned and, in some code paths, authored.

There is currently no single Harness source of truth. Effective behavior is assembled from several sources with different reachability and durability.

## Configuration-source map

| Source | What it contributes | Authority/reachability |
| --- | --- | --- |
| `conversation_harness_catalog.json` | ten base role profiles: identity, prompt, skills, sandbox, approval, MCP base tools, completion criteria | compile-time embedded and productive |
| `conversation_harness.rs` | validation, profile decoding, and code-authored stage-specific effective configurations | productive; creates variants absent from static catalogue |
| `conversation_harness_working_copy.rs` | rich mutable configuration envelope and optimistic draft commands | durable backend exists; not exposed through product Tauri commands |
| `conversation_harness_revision.rs` | immutable revision lineage and verified content-addressed local objects | productive for Work Unit execution; only partially inspectable in UI |
| `.agents/product-skills/` | role-specific operational guidance and metadata | shipped product assets; availability depends on Codex discovery |
| application-generated prompts | exact stage instructions and supplied context | productive runtime behavior, distributed through services |
| MCP tool descriptions/server instructions | endpoint semantics and prohibitions | productive runtime behavior, colocated with endpoint code |
| dynamic Codex configuration | server URLs/tokens/tools, sandbox/network settings, model/options | productive per invocation |
| frontend Harness Management model | generic inspection/editor experience | mounted read-only for Plan Builder; recorded source simulates richer behavior |

## Static base-role catalogue

The compile-time JSON defines ten profiles:

| Key | Version | Base responsibility |
| --- | ---: | --- |
| `epic_plan_builder` | 4 | discuss and submit an Epic proposal; request initiation |
| `epic_bootstrap_generator` | 3 | generate bounded bootstrap materials |
| `epic_runner` | 3 | own the initiated Epic and select a Sprint |
| `epic_runner_escalation_reassessment` | 2 | reassess escalated Sprint concern |
| `sprint_runner` | 2 | own a selected Sprint |
| `sprint_runner_planning_control` | 1 | request the current Work Slice Planner |
| `sprint_runner_handback_reassessment` | 1 | disposition a no-progress handback |
| `work_slice_planner` | 2 | read context and propose/refine/complete a Work Slice |
| `work_unit_handler` | 2 | own a Work Unit |
| `work_unit_implementer` | 3 | perform one isolated implementation attempt |

Common configuration facts:

- model and reasoning effort are `null`, leaving provider/user defaults influential;
- approval policy is `never`;
- context is delivered in the first query;
- sandboxes are read-only except the original WorkspaceWrite Implementer;
- skills are named and validated as repository assets;
- the catalogue declares only the base-role tool set, not every continuation tool.

## Code-authored runtime variants

The Work Unit lifecycle automatically creates and pins five distinct immutable configurations:

| Logical role | Runtime variant | Tools |
| --- | --- | --- |
| Handler | original baseline | none |
| Handler | action continuation | `request_work_unit_implementer` |
| Handler | independent review continuation | read/accept/return review tools |
| Implementer | original execution | none |
| Implementer | reporting continuation | submit/complete outcome tools |

Pre-start, start and started Sprint continuations similarly add tools absent from the base Sprint Runner profile. These variants are product policy, but they are created from Rust functions and durable revision history rather than enumerated in the static role catalogue.

## Durable working copies and revisions

The working-copy envelope is broader than the compiled catalogue and models:

- agent and visual identity;
- prompt-prefix delivery;
- skill discovery policy and entries;
- tool discovery/schema boundary;
- model policy and allowed models;
- reasoning choices;
- sandbox and approval;
- hooks;
- update policy.

Immutable revisions are stored in two coordinated forms:

- SQLite lineage, commands and publication evidence;
- content-addressed configuration objects and JSON commit manifests under `harness-revisions/`.

Despite names such as `repository_commit_ref`, this is not Git. Reads verify the SHA-256 object, manifest, normalized configuration, predecessor and local reference.

## Product skill boundary

Seven canonical product skills live under `.agents/product-skills/`, each with a `SKILL.md` and `agents/openai.yaml`. Harness construction validates that the named file exists and its metadata matches, adds discovery/use guidance to the prompt, and runs the child from the repository root.

The application does not itself load or activate the skill. Availability still depends on Codex discovery, so configured, discoverable, selected and followed are separate evidence stages.

An older parallel `.agents/skills/` vocabulary remains in the baseline, including earlier overall-plan, plan-step and slice-plan concepts. Current `main` has materially rewritten skill assets while lacking much of this engine line. Skill inventory is therefore branch-sensitive historical evidence as well as application configuration.

## Harness Management frontend reality

The generic `ConversationHarnessInspector` experience includes models for editing, saving, committing, publishing, queueing, identity, model policy, skill/tool catalogues and version history. The mounted product source does not provide those capabilities.

Its only backend query is `load_managed_plan_builder_harness_inspection`. The adapter maps the compiled Plan Builder profile into the generic view and hardcodes:

- no working copy;
- version history/activation not connected;
- Session binding untracked;
- model preference/resolution not connected;
- skill/tool/model catalogues not connected;
- one “Compiled Harness profile” version.

The durable backend authoring/revision system has no Tauri command surface. Work Unit Sessions can use pinned durable Harness revisions while the generic inspector still reports non-Plan-Builder Sessions as unbound. The richer interactive behavior currently comes from a recorded development source.

## Drift and duplication points

- `managedPlanBuilderSession.ts` retains obsolete Plan Builder tool names.
- static catalogue tool lists do not describe stage-specific runtime variants.
- prompts, skills, tool descriptions and server instructions repeat overlapping prohibitions.
- model/effort defaults influence execution but remain unresolved in the inspector.
- skill policy is partly data and partly prompt guidance, without demonstrated provider-side enforcement.
- a generic Harness Management UI is backed by a Plan Builder-specific command.
- code automatically authors revisions even though the product does not expose Harness authoring.

## Product and architectural interpretation

- “Harness” currently names at least three things: a base role template, an immutable effective revision, and a frontend management concept.
- Effective configuration should be traceable per invocation: base profile, derived variant, pinned revision, dynamic injection and observed runtime options.
- The durable revision system appears more expressive than the mounted management surface and more operationally authoritative for Work Units than the static inspector suggests.
- Centralization should not erase stage-specific least-authority variants. It should make those variants explicit, inspectable and generated from one governed model.

## Questions for later disposition

- Is the compiled JSON intended as seed data, permanent base policy, or the long-term canonical source?
- Should code-authored variants become first-class catalogue entries or a typed derivation model?
- Which working-copy fields are real future product requirements versus exploratory UI breadth?
- Should Harness Management be a product authoring surface, an inspector, or an operator tool?
- How will a user know the effective model, reasoning, sandbox, skill discovery and MCP tool set for one invocation?
- Which historical `.agents/skills/` assets can be removed once branch integration is understood?
