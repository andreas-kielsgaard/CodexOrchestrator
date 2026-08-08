# Operation trace: Plan Builder proposal and initiation

The Plan Builder combines a product-managed Agent Session, an invocation-scoped MCP server, durable proposal storage, and a shared human confirmation boundary.

## Discussion and build requests

1. `EpicPlanBuilder` uses a Plan Builder-specific client while retaining the shared Agent Session reads, updates, transcript, and workspace.
2. `createTauriManagedPlanBuilderSessionClient` replaces only two send operations:
   - ordinary discussion invokes `send_managed_plan_builder_message`;
   - the Build action invokes `request_managed_plan_builder_action` with application-owned prompt text.
3. `orchestration/transport.rs` converts these calls into `ManagedPlanBuilderService::send` and `request_plan`.
4. `request_plan` records application provenance and supplies `Build the epic plan based on what we have discussed`; ordinary discussion retains user provenance.

## Managed launch preparation

`ManagedPlanBuilderService`:

- resolves or establishes the durable planning draft and Session association;
- loads the `epic_plan_builder` Harness profile;
- starts a managed MCP invocation;
- creates a `RuntimeLaunchExtension` containing generated Codex configuration, a child-only bearer environment value, and the application prompt prefix;
- sends through the shared Agent Session lifecycle;
- binds the durable Agent invocation identity to the MCP authority;
- retains the server until terminal notification, then stops it;
- separately consumes or releases claimed Plan Builder context based on durable launch-acceptance evidence.

## MCP transport

`src-tauri/src/orchestration/mcp.rs` starts an in-process streamable-HTTP server:

- binds an ephemeral `127.0.0.1` port;
- exposes `/mcp`;
- uses a fresh bearer token referenced through a generated environment-variable name;
- checks bearer, exact Host, and allowed Origin before requests reach `rmcp`;
- owns cancellation and thread shutdown for the invocation.

`CodexMcpInjection` renders the server as `-c mcp_servers.*` values for the child Codex CLI, including enabled tools, required status, approval mode, and timeouts.

## Proposal tool

`submit_epic_plan_proposal` accepts only the concise typed Sprint projection. `PlanBuilderInvocation` derives draft, profile, association, actor, revision, and replay authority that the model cannot supply.

The tool calls `OrchestrationApplication::save_epic_plan_proposal`, which delegates to the SQLite orchestration repository. Success reports persisted or idempotent replay; it is not inferred from transcript text or invocation completion.

## Initiation tool

`request_epic_initiation`:

1. Requires the exact bound Agent invocation.
2. Captures the current durable proposal revision precondition.
3. Derives an application-owned idempotency identity.
4. Requests confirmation through `InitiationConfirmationCoordinator`.
5. Emits the shared Tauri confirmation event for the frontend modal.
6. Waits up to 240 seconds for explicit resolution.
7. On confirmation, applies durable initiation through the orchestration application/repository.
8. Reports only the projected initiation. Bootstrap materials and Epic Runner launch remain later states.

The frontend also has direct confirmation commands for button-originated initiation. Agent- and button-originated requests share the coordinator and durable application semantics.

## Configuration involved

- `conversation_harness_catalog.json`: Plan Builder context, skill guidance, read-only sandbox, approval policy, enabled tools, and completion criteria.
- `conversation_harness.rs`: loads and validates the compiled profile.
- `mcp.rs::CodexMcpInjection`: renders Codex CLI configuration and bearer environment linkage.
- `.agents/product-skills/epic-plan-builder/SKILL.md`: canonical skill source named by the Harness profile.
- Agent Session runtime options and working-directory selection.

## Architectural reading

The flow demonstrates a strong application-authority design: model input is intentionally narrow and hidden identities are derived server-side. It also shows configuration duplication and infrastructure repetition: tool names and policy appear in the catalog, Rust validation, MCP router, generated CLI configuration, frontend compatibility declarations, and tests.
