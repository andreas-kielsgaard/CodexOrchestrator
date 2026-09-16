# Job Agent OTP designer preparation

Planning only. Target branch: codex/workflow-continuation-files at 8ef084b.

## Objective

Prepare a locally configured Job Agent OTP so a Workflow author can use the current Job Agent MCP tools and grants while designing a source-setup workflow. Completion means Technical Settings shows a verified Job Agent package; node capability configuration can select its tools and grants; Workflow continuation carries structured data; and a draft can save, activate, and compile.

This slice does not create the source-setup recipe or replace the Job Agent web workflow.

`main` at a806212 does not include this OTP work. Preserve this worktree's existing untracked Codex defaults plan and main's unrelated documentation changes. The Job Agent repository is dirty on `feature/mcp-local-harness`; retain its existing MCP work.

## Boundaries

| Owner | Responsibility |
| --- | --- |
| Orchid | Recipes, routing, prompts, session creation, Harness mediation, session-scoped upstream lifetime, and persisted deliveries. |
| Job Agent OTP | Job Agent tool metadata, selected-tool to capability mapping, and node grant validation. |
| Job Agent | Local listener, bearer, source operations, validation, revisions, readiness, and mutation grant enforcement. |
| Orchid installation settings | The selected Job Agent root and Python executable. |

The package receives no repository, Tauri state, database connection, WorkflowExecutionService, or direct Session object. The existing LocalCodexHarness is excluded because it creates a separate Codex process; Orchid owns the agent Session.

## Chosen contracts

### Agent MCP server declarations

Extend PackageDescriptor with `agentMcpServers`. These describe MCP services that Orchid can put behind the Harness proxy. They are distinct from the existing OTP tools, which remain triggers, session-event consumers, or destination actions.

The Job Agent package declares one `job_agent` server with current concrete tools, descriptions, and their `McpCapability`. The Designer groups these choices under Job Agent. The host starts the smallest capability union, and the Harness proxy exposes only the selected tools.

### Node grant configuration

Add `agentMcpConfiguration` to a Workflow node, keyed by package and server. The Job Agent value stores a list of named grants such as `source_registry:configure`, `source_calibration:calibrate`, `recipe_authoring:author_recipe`, `recipe_authoring:approve_recipe`, `recipe_authoring:adopt_recipe`, `source_execution:configure`, `source_execution:test`, and `source_execution:enable`.

Use a declared multi-select field. Grants are intentionally unscoped in this slice because a new source has no ID before creation. The graph separates grants by node. Source-ID expressions, approval-derived grants, and engine-enforced approval remain deferred.

### Session upstream lease

Add a product-owned session-scoped MCP upstream lease. It is created after Orchid has the session identity and before Harness binding, resolves only for that session, retains the external host owner, and stops on retirement or product shutdown. Static upstreams retain their current behavior.

### Structured activation data

Add optional `data: object` to a Workflow entry request and to `trigger_workflow_continuation`. Continuation emits `outputFiles`, trusted `sourceNode`, and `data`. Data is one selectable connection field, remains structured in delivery records and prompts, and has no source-specific schema or global variable store.

A source workflow can begin with a source draft object and carry source, artifact, candidate, and revision IDs explicitly in later continuation data. Node prompts define that convention.

## Work packages

### JA-01 Job Agent bridge process

Create `app/code/job_agent/mcp/orchid_host.py`. It provides a versioned describe response and a standard-input controlled serve mode. Serve receives root, selected capabilities, and grants; starts the existing McpHost; returns only the loopback URL, fresh bearer, server name, and selected capabilities; and stops on input close or process termination.

Adapt `app/code/job_agent/mcp/contracts.py` so the tool-to-capability catalogue is one Python source used by both `create_server` and `describe`. Adapt `app/code/job_agent/mcp/__init__.py`, `app/code/job_agent/cli.py`, and `docs/mcp-local-harness.md` only to expose and document this bridge. Add tests beside `tests/test_mcp_server.py` for manifest parity, malformed control input, selected capability and grant forwarding, bearer secrecy, and shutdown. Do not add a second listener or use LocalCodexHarness.

### OR-01 OTP API and catalogues

Adapt `src-tauri/src/otp_api/contract.rs`, `handles.rs`, and `mod.rs`; `src/application/otp/contracts.ts`; authoring catalogue transport; DTO fixtures; and contract tests. Add `AgentMcpServerDescriptor`, `AgentMcpToolDescriptor`, and declared binding configuration fields. Add the narrow `MultiChoice` configuration type needed for grants. Do not add arbitrary schema forms.

### OR-02 Package installation settings

Create `src-tauri/src/otp_host/installations.rs` with its repository and Tauri transport. Persist the Job Agent root and Python command in the active product database through `src-tauri/src/storage.rs` migrations. Adapt `active_app.rs`, `otp_host/mod.rs`, `TechnicalSettingsScreen.tsx`, `OtpConfigurationPanel.tsx`, and matching application and infrastructure clients.

Technical Settings offers root and Python fields and Save and verify. Verification runs the JA-01 describe command and requires a match with the compiled Job Agent OTP declaration. It records configured, unconfigured, or incompatible truthfully; it does not become a package manager or load arbitrary packages.

### OR-03 Compiled Job Agent OTP and host adapter

Create `src-tauri/src/otp_packages/job_agent/mod.rs`, `manifest.rs`, and `tests.rs`, plus `src-tauri/src/otp_host/job_agent.rs`. Register it in `otp_packages/mod.rs` and import Workflow plus Job Agent from `active_app.rs`.

The pure package exposes the static current Job Agent catalogue and grant choices, validates node configuration, and requests no product handles. The host adapter reads the verified installation, selected node tools, and grants; starts JA-01; validates the ready response; maps grants to McpMutationGrant; and returns a managed upstream lease. It does not validate source lifecycle decisions.

### OR-04 Session-specific Harness provisioning

Adapt `harness_engine/domain.rs`, `service.rs`, and `proxy.rs`; `otp_host/session_control.rs`; `workflows/compiled_plan.rs`; and `workflows/execution.rs`. Add an `otp_host` provisioning coordinator that receives trusted instance, node, and session context plus selected MCP tools and node binding configuration.

Replace dynamic global latest-server resolution with session-scoped lease lookup while preserving static Workflow resolution. Persist the mediated binding and selected configuration only, never the bearer or loopback descriptor. Test concurrent Job Agent sessions, launch failure cleanup, lease release, shutdown, and static Workflow regression.

### OR-05 Structured entry and continuation data

Adapt `otp_packages/workflow/tools.rs` and `mod.rs`; `workflows/execution.rs`, `compiled_plan.rs`, `compiler.rs`, `prompt_content.rs`, `instances.rs`, and `execution_transport.rs`; then matching TypeScript contracts and Tauri clients.

`DispatchWorkflowUserRequestInput` gains optional data. The base prompt action receives text plus separately referenced JSON data. `trigger_workflow_continuation` accepts, validates, emits, and declares data. Preserve `outputFiles` and `sourceNode`. Declared paths never establish file authorship.

### OR-06 Designer preparation

Adapt `OtpMcpToolsPicker.tsx`, `CapabilitySetFields.tsx`, `WorkflowNodeEditor.tsx`, `OtpConfigurationEditor.tsx`, `WorkflowConnectionEditor.tsx`, `WorkflowInstancePanel.tsx`, and their DTOs, clients, fixtures, and tests.

Reuse `OtpElementPicker` to group Job Agent tools and show their descriptions. Do not show those tools as connection triggers or destination actions. When a node selects Job Agent tools, show the declared Job Agent grants directly below the MCP selector and persist them in `agentMcpConfiguration`. Existing unavailable-choice behavior remains.

The instance panel adds an optional Structured data JSON object. The connection editor lists continuation data beside `outputFiles` and `sourceNode`. A visible happy-path authoring exercise configures blank nodes with Job Agent tools and distinct grants, connects them with Workflow continuation plus data, saves, activates, and compiles. It does not create a source or call a mutation tool.

## Validation

Job Agent: focused pytest for bridge protocol plus existing MCP tests; one bridge integration starts serve, calls `tools/list` through the bearer-protected listener, and proves EOF stops it.

Orchid: package and installation tests; Workflow compile and data tests; Harness lease isolation and cleanup tests; frontend tests for installation state, OTP grouping, grant persistence, and JSON data validation. Run `cargo build --manifest-path src-tauri/Cargo.toml --profile test-fast --bins` and `npm run build`.

Visible evidence configures the local installation, creates and compiles a Job Agent-enabled draft, then starts one disposable read-only node session and inspects its mediated profile. Do not run source mutation, testing, or enablement in this slice.

## Deferred

The actual source-setup recipe and prompts; Job Agent UI replacement or launch/link API; deterministic action destinations; source-ID expressions; approval enforcement; revision coordination; dynamic result-to-data mapping; external package distribution, hot reload, remote installs, and broader security work.
