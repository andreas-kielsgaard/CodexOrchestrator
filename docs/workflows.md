# Workflows and Session Events

A Workflow describes how work is delivered between Agent Sessions. A recipe defines nodes, their configuration, and connections. An instance captures an activated recipe and a chosen worktree. The Workflow compiler translates those instructions into **Session Events**, whose generic service selects sessions, assembles prompts, dispatches invocations, and records delivery results.

This guide describes main `60c3798` (14 September 2026). The replacement implementation is mounted and integrated. [Agent Sessions](agent-session/README.md) owns conversation/runtime behavior, and [execution configuration](execution-configuration.md) owns profile lifetimes and direct versus managed choices.

## Author and activate a recipe

The Workflow authoring view is a flow canvas with node and connection editors. Nodes have stable IDs, names, positions, a referenced Capability Profile, embedded Node Profile values, optional initial prompt, and optional Agent Identity. Copying node configuration creates independent values; it does not make one node a live reference to another.

A connection specifies its source and destination nodes, trigger, ordered prompt inputs, fixed prompt text, and destination-session selection. The editor constrains choices to supported combinations. Activation requires a valid starting node, valid endpoints and profile/default choices, and compatible trigger/prompt inputs.

Editing, saving, and activation are separate:

- A working draft can differ from the saved revision.
- Save captures a version of the working draft; a late reply must not overwrite newer typing.
- Activate names the expected saved revision. It does not silently save or erase unsaved edits.
- The application-owned draft workspace retains in-app edits across route changes. It is not a cross-process autosave or crash-recovery system.

The graph components own layout and gestures. The authoring screen owns recipe edits; typed clients and backend services own persistence and validation. Shared profile rules are explained in [configuration layers](execution-configuration.md#configuration-layers).

## Create an instance and begin work

1. Save and activate the intended recipe revision.
2. Create a named instance and select an existing repository/branch/worktree target through the shared repository catalog.
3. Open the instance's graph and select the node whose conversation you want to begin.
4. Submit the first request. The application resolves or creates that node's session and invokes it in the instance worktree.
5. Continue in the embedded Session view and inspect connection attempts and delivery records as work moves.

Creating an instance stores its name, exact recipe contents/revision, target, and creation time. It creates no Agent Session, starts no provider invocation, and does not create or modify the worktree. A changed active recipe is rejected if it no longer matches the creation request's expected revision. Later recipe edits do not rewrite an existing instance.

The recipe has one starting-node marker. The default entry uses it; the current instance UI and command can also address an explicitly selected valid node. Starting is a human delivery to a node, not a separate durable Workflow launch lifecycle.

The instance graph reuses shared graph presentation with authoring. Its nodes and connections lead to sessions, read-only configuration, and attempts. Embedded conversations use the same profiled Session workspace as the standalone view. This preserves one conversation implementation while keeping Workflow navigation and execution context with the host.

The stored target supplies the execution directory. Workflow does not add branch switching, worktree creation, cleanup, or enforcement against later external branch movement. [Repository and Worktree Review](worktree-review.md) owns registration, live Git observations and review operations.

## Resolve and deliver

There are four distinct objects:

| Object           | Meaning                                                                                                                                             |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Event definition | Reusable trigger, target selection, prompt-source definitions and optional creation configuration compiled from the recipe.                         |
| Event occurrence | One concrete user request, successful invocation, managed MCP call, or application event with correlated source data.                               |
| Event group      | The recorded result of applying one occurrence/definition, including selected sessions and overall delivery outcome.                                |
| Delivery         | One target session, the exact prompt contributions used, and either an accepted runtime launch with its invocation reference or a dispatch failure. |

The execution sequence is:

```text
stored instance and scoped occurrence
  -> compile definitions from the stored recipe
  -> record the Workflow attempt
  -> resolve referenced prompt content
  -> materialize the Session Event
  -> find existing target sessions
       missing + create -> resolve birth configuration and store session/address
  -> dispatch to each selected session
  -> store group/delivery results and notify readers
```

Workflow source references identify the recipe, instance, node or connection. Generic Session Events carry namespace/kind/ID references and an opaque creation configuration; they do not need Workflow imports, Git knowledge, provider configuration or role semantics. The Agent Session adapter interprets its own creation envelope and stores the session/profile/address together.

**Target lookup happens before birth configuration resolution.** Existing sessions use their stored Session Profile. Current shared profile values are resolved only when a new session must be born. An edited or deleted Capability Profile can therefore affect a new node session without blocking an already-created target solely because its old catalog entry changed.

Workflow membership comes from the logical address: instance scope plus node subject. The instance view queries that address directory. It does not maintain a competing mutable membership table or infer ownership from session titles.

## Destination selection and prompt sources

The generic targeting contract supports these controls; Workflow connections apply them to the destination node's logical address.

| Control             | Choices and meaning                                                                     |
| ------------------- | --------------------------------------------------------------------------------------- |
| Target              | An exact Session reference or a logical scope/subject address.                          |
| Cardinality         | First matching session or all matches.                                                  |
| Ordering            | Newest-created or most recently addressed.                                              |
| Running filter      | Any, running only, or not running.                                                      |
| Creation provenance | Optional creating-event and/or creating-session filter.                                 |
| Missing target      | Create, fail, or no-op. Creation requires a logical address and creation configuration. |

A running filter selects candidates; it does not implement a queue or override the Session rule of one active invocation. A matching target can still reject a dispatch, and that failure is recorded.

Prompts preserve the configured contribution order. Supported sources include user text for an entry request, the exact completed invocation's final output, a named MCP argument, a named application-event field, and referenced content. A connection's fixed text follows its configured inputs. The destination node's initial prompt is included only when this delivery creates the session.

Workflow file references use `file/path` and read text relative to the instance worktree. Canonical path checks reject escapes; missing, unreadable or unsupported content fails before receiver launch. A path supplied as MCP text is not automatically read as file contents. Fixed node/connection prompts retain references to the recipe revision and field from which their delivered text came.

Materialized references and delivered text are evidence of what was supplied. They are not general filesystem-access authority, file authorship, or semantic acceptance of a produced artifact.

## Triggers, trusted context and results

Current routing supports:

- **User request:** the instance/node entry operation.
- **Successful invocation completion:** the normal persisted Session terminal notification, scoped to its Workflow address. Failed, canceled and interrupted turns do not fire this success trigger.
- **Managed MCP handoff:** the application-provided `workflow_handoff/handoff_to_agent` tool using `prompt_agent_files_and_text/v1`.
- **Explicit application event:** a scoped service entry with a concrete event kind and supplied fields. It does not create an automatic generic background producer.

For completion and managed-source routing, Workflow resolves the source instance/node from the trusted Session address and considers its outgoing connections. Each matching connection is independent: one source event may dispatch through several connections. This is fan-out, not a joined condition requiring all upstream work to finish.

The managed MCP proxy binds allowed exposure before launch and supplies trusted session/invocation context. Call arguments cannot invent that authority or redirect the caller to another instance. The tool handler validates the source and interface at its own boundary. Seeing arbitrary MCP traffic does not make it a Workflow trigger. [Harness delivery](execution-configuration.md#harness-configuration-and-delivery) explains the shared technical binding.

A Workflow attempt exists before content preparation and dispatch. It can record failure before a Session Event group exists. Repeated normal source notifications use a stable occurrence/connection identity so they do not launch the same attempt again. This is not an automatic retry queue or a general crash-safe exactly-once guarantee. A failed handoff leaves the sender's terminal fact unchanged.

A delivery marked **Dispatched** means the Session adapter accepted its runtime launch and returned an invocation reference. An event group marked **Delivered** means its deliveries were dispatched successfully; **PartiallyDelivered** and **DeliveryFailed** summarize dispatch failures. **NoTarget** and **Noop** record the configured missing-target result. None of those labels says that all recipients completed their work or that the product accepted their output.

Group-completion triggers remain disabled at Workflow activation. Whether “group complete” should mean dispatch finished or every recipient invocation finished, and how failures count, is unresolved. Steering an active Session is also explicitly excluded from Workflow delivery routing.

Session Event and instance readers refresh after their respective durable records change. Their notification streams are separate because a preparation failure may update an instance without creating a delivery.

## What this engine does not decide

Workflow routing does not itself implement Epic/Sprint planning authority, human approval, Work Unit review, accepted integration, or settlement. Files and final messages are inputs or evidence; those product effects need explicit semantic operations owned by the [orchestration system](orchestration/README.md).

The new path replaced Workflow V1's Role-based executor, commands, UI fallback and runtime tables. Current instances and attempts use `workflow_recipe_instances` and `workflow_recipe_attempts`; Session Event groups/deliveries and Agent Session addresses have their own owners. Old V1 node-session coordinators, effective-recipe tables, and launch-state plans are historical implementation material.

The user explicitly chose functional replacement with disposable old Workflow data rather than compatibility behavior. That superseded the earlier repair phase's narrower additive-migration scope. It did not retire every retained Epic/Sprint or Harness consumer elsewhere in the application.

## Implementation ownership

| Responsibility                                                   | Current source                                                                                                                                                                                                                                                                         |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Recipe contracts, save/activation and compilation                | [`workflows/authoring.rs`](../src-tauri/src/workflows/authoring.rs), [`authoring_service.rs`](../src-tauri/src/workflows/authoring_service.rs), [`compiler.rs`](../src-tauri/src/workflows/compiler.rs)                                                                                |
| Stored instances, attempts and execution orchestration           | [`workflows/instances.rs`](../src-tauri/src/workflows/instances.rs), [`execution.rs`](../src-tauri/src/workflows/execution.rs), [`execution_transport.rs`](../src-tauri/src/workflows/execution_transport.rs)                                                                          |
| Typed source/address references and content                      | [`address_references.rs`](../src-tauri/src/workflows/address_references.rs), [`prompt_content.rs`](../src-tauri/src/workflows/prompt_content.rs)                                                                                                                                       |
| Completion/application sources and managed handoff               | [`event_sources.rs`](../src-tauri/src/workflows/event_sources.rs), [`mcp.rs`](../src-tauri/src/workflows/mcp.rs)                                                                                                                                                                       |
| Generic materialization, target selection, groups and deliveries | [`session_events/`](../src-tauri/src/session_events/)                                                                                                                                                                                                                                  |
| Actual Session creation/address persistence/invocation           | [`agent_sessions/session_event_adapter.rs`](../src-tauri/src/agent_sessions/session_event_adapter.rs), [`agent_sessions/repository/`](../src-tauri/src/agent_sessions/repository/)                                                                                                     |
| Authoring, instance and shared graph UI                          | [`features/workflowAuthoring/`](../src/features/workflowAuthoring/), [`workflowInstances/`](../src/features/workflowInstances/), [`workflowGraph/`](../src/features/workflowGraph/)                                                                                                    |
| Draft lifetime and typed clients                                 | [`components/draftWorkspace.ts`](../src/components/draftWorkspace.ts), [`application/workflowAuthoring/`](../src/application/workflowAuthoring/), [`workflowInstances.ts`](../src/application/workflowInstances.ts), [`application/sessionEvents/`](../src/application/sessionEvents/) |

## Decision provenance and evidence

- **The replacement model:** “Workflow to Session Event Overhaul”, task `01a0390e-64bb-7b01-b277-e019f37d51bc`, original rollout `rollout-2026-08-25T15-13-59-01a0390e-64bb-7b01-b277-e019f37d51bc.jsonl`, user messages at lines 3070, 3132 and 3192 establish generic events, typed references, creation-time configuration and narrow concrete interfaces. Line 13451 authorizes replacement behavior and disposable old data. Historical sources include `e2bfc6c:docs/session-event-model/conceptual-model.md` and `e2bfc6c:docs/orchestration/workflow-instance-execution-plan-2026-08-25.md`.
- **Shared graph and conversation UI:** the same task's user messages at raw lines 9106, 11616 and 12025 require restoring the node-based instance view and reusing related components. The original regression and later graph evidence remain in the [regression package](regression-review/README.md); the [walkthrough package](ux/session-event-model-walkthrough/README.md) contains older list-style captures.
- **Bounded repair and integration:** “Harness Overhaul: Regression test”, task `01a07aa8-4bdb-7ba1-923c-cd444ad477fc`, turn `01a07bc3-45a1-7fa0-b22a-c96ee742edf2` records the scope-creep correction; turn `01a07c65-4d42-7d60-8202-e43149bd168f` records repair publication. Overhaul turn `01a082f3-c42a-70e1-8b68-674233803264` records main integration at `deecb2f`. The earlier repair status is retrievable at `e2bfc6c:docs/regression-review/repairs/README.md`.
- **Routing versus product semantics:** “Workflow: Replicating Epic Workflow”, task `01a04442-9286-7aa3-a393-ebe6cedf0624`, turn `01a04442-957d-7591-8791-0aceb3ebfd63` is the original source analysis. Its distinction remains useful; its V1 inventory and suggested future components are not current implementation decisions.

[Validation evidence](validation-evidence.md) separates compiler tests, real application/SQLite tests with fake inference, browser fixtures, native observations and provider proof. A restored graph screenshot or passing compiler test alone does not establish end-to-end Workflow execution.

## Separate OTP development

The Orchestration Tool Package implementation at `8ef084b` is outside main. Its direction is a curated package API with narrow node/session handles while the product retains event observation and routing. Package selection/settings UI and `trigger_workflow_continuation` belong to that branch, not to the managed handoff API described above.

The original task **“Orchestration Tool Package”**, `01a080f8-6b8d-79f0-8307-294e1ae73d3a`, rollout `rollout-2026-09-08T14-22-39-01a080f8-6b8d-79f0-8307-294e1ae73d3a.jsonl`, user messages at lines 9, 45, 65 and 230 establish that boundary and the instruction not to pre-develop unused capabilities. Line 3388 specifies tool-declared interfaces and consumer compatibility, with reusable configuration UI first and package-provided UI later. Subsequent compatibility and apply/save questions remain open; the discussion is not an adopted universal plugin framework.
