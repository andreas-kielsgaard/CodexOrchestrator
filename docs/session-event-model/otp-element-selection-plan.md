# OTP element selection and Stop session

Revision 1, 2026-09-08. Planning only; implementation has not started.

## Objective and baseline

Provide one reusable OTP selection modal with trigger-output, destination-action and profile-MCP adapters. Add Stop session to the Base Workflow OTP. Workflow entry uses the same destination-action adapter, catalogue and execution rules as a connection destination.

Inspected `codex/workflow-continuation-files` at `532e559` in `C:/Users/user/.codex/worktrees/workflow-continuation-files`. The checkout was clean before creating this plan. Prior OTP validation is recorded in `otp-exploration-validation.md`; this planning pass ran no tests or provider sessions.

Completion: all three selectors work through the shared modal; entry and connections can configure either destination action; Stop session reaches the existing cancellation service with a recorded outcome; existing continuation, prompting and profile constraints still work. Deliver focused automated checks and a visible walkthrough. No implementation or commit is authorized by this planning request.

## Decisions, assumptions and concerns

Accepted: expandable OTPs in a narrow left column, relevant child elements, click to inspect details, explicit confirmation; single selection for outputs/actions, multiple selection for MCP tools. No separate entry-action category. Stop session replaces the proposed additional prompting action.

Proposed minimum stop behavior: select one running session of the bound destination node, newest by default, with the existing last-addressed ordering available. No matching active session is a recorded no-op. Stop requests cancellation of current inference; it does not delete/archive the session. Selection happens at execution time so recipes do not depend on instance-specific session IDs. Exact-ID editing, stop-all and additional filters are deferred. These defaults remain a planning assumption, not a newly accepted user requirement.

| Concern | Current evidence and impact | Resolution / uncertainty |
| --- | --- | --- |
| Shared selection | `WorkflowConnectionEditor.tsx` flattens outputs/actions into dropdowns; profile tools use `CatalogMultiSelect`. Local, reversible UI change. | P1/P2: one modal, three typed adapters; tests exercise real selection behavior. |
| Item identity and ownership | Outputs have package/tool/output IDs; tools have package/tool IDs. The current managed MCP server is explicitly named after its package in `otp_host/mcp.rs`. | P1/P2: preserve IDs and use that declared mapping, not matching display labels. No Harness metadata redesign. |
| Stop execution | OTP tools return prompt session requests; `AgentSessionApplication::cancel_invocation` already owns cancellation. Moderate runtime scope and race risk. | P3: add a narrow typed stop request and host adapter, pin the invocation being stopped, record request versus terminal state separately. |
| Entry parity | Entry currently supplies `{}` configuration and demands exactly one delivery. Merely reusing a selector would not make Stop work. | P4: shared action/configuration semantics, aggregate action result, allow no prompt/no delivery. |
| Profile constraints | Runtime exposure and profile restrictions determine available tools independently of package discovery. | P2: intersect eligibility, retain unavailable stored choices visibly, preserve non-OTP MCP tools. |
| Small catalogues and modal usability | Only one real package is installed; outputs can share tool names. | P1: zero/one states, synthetic two-package fixture, keyboard/focus and screenshot verification. No new production package for demo purposes. |

Scope is bounded to these selectors, cancellation and entry parity. The main coupling is the shared OTP descriptor/result contract across Rust, TypeScript, the compiler and instance UI. Source and test seams are available locally. Live cancellation is a validation gate; setup must use existing product services/CLI handles where available, reserving computer use for UI evidence. Stop only a disposable test invocation.

## Work packages

### P1 — Reusable OTP picker and shared catalogue types

Outcome: package browsing independent of Workflow or profile mutation.

- Create `src/components/otp/OtpElementPicker.tsx`, `otpElementPicker.css` and `OtpElementPicker.test.tsx`. Use a native dialog with accessible labeling, Escape/cancel, focus containment and return to the invoking button.
- Inputs: package groups, stable item IDs, current selection, single/multiple mode, detail renderer and confirm callback. Keep preview selection separate from checked choices; cancel changes nothing. The picker neither fetches data nor persists configuration.
- Clicking a package expands its eligible elements. Clicking an element shows details. Reopening reveals the current selection. Zero options explains the empty state; one option still supports inspection. Unavailable saved choices remain visible and are not silently replaced.
- Move OTP DTOs out of `src/application/workflowAuthoring/contracts.ts` into `src/application/otp/contracts.ts` with an index export. Add `src/features/otp/otpElements.ts` for shared descriptor projection and details. Avoid a general plugin framework or recursive schema editor.
- Acceptance: two packages with duplicate labels remain distinct; multi-select, confirm/cancel, unavailable choices, zero/one options and keyboard/focus behavior. No backend dependency beyond the agreed descriptor shape.

### P2 — Three adapters and mounted selectors

Outcome: each existing selection surface uses P1 while retaining its own rules.

- Create `WorkflowTriggerPicker.tsx` and `WorkflowDestinationActionPicker.tsx` under `src/features/workflowAuthoring/`, plus `OtpMcpToolsPicker.tsx` under `src/features/executionConfiguration/`.
- Edit `WorkflowConnectionEditor.tsx` and `otpPresentation.ts`: replace the two selects with current-selection summaries and Set/Change buttons. Trigger details identify the specific output, emitting tool and offered fields. On confirmation reconcile stale output-field inputs; show which would be removed before applying. Changing an action initializes its configuration only when the action actually changes.
- Keep `OtpConfigurationEditor.tsx` as the configuration renderer used by the destination adapter. The same adapter is mounted for the initial user-request destination in P4.
- Edit `CapabilitySetFields.tsx`, `types.ts`, `presentation.ts`, `ExecutionConfigurationScreen.tsx` and `WorkflowAuthoringScreen.tsx`; pass the catalogue through capability/node editors where needed. Reuse the existing `list_workflow_capabilities` endpoint via an injected catalogue reader wired in `App.tsx`, rather than introducing another backend catalogue.
- MCP eligibility is the intersection of imported MCP descriptors, runtime exposure and existing profile restrictions. Package/server identity follows the current explicit registry mapping. Non-OTP runtime tools remain selectable in a clearly labeled MCP-server group, without being presented as OTPs. Keep persisted server/tool values unchanged.
- Acceptance: trigger/action save-reload and cancel behavior; capability and node profiles both use the multi-picker and preserve restrictions/non-OTP values. Focused tests belong beside the adapters and in existing `WorkflowContinuation.test.tsx` and execution-configuration editor tests.
- Dependencies: P1; final destination catalogue integrates P3. Do not convert node, field, file or session selectors into OTP browsers.

### P3 — Base Workflow OTP Stop session

Outcome: a package-owned action requests cancellation through product-owned session handling.

- Create `src-tauri/src/otp_packages/workflow/stop_session.rs`; update `workflow/mod.rs` and package tests. Declare `stop_session` as an Action, not a new MCP endpoint or trigger. Use existing `OtpHost::sessions` data to select the target. Add only newest/last-addressed ordering; no creation or broad session-selection framework.
- Extend `otp_api/contract.rs` with a typed `SessionStopRequest { node_id, session_id }`, a declared stop-request output kind and an additive `ToolResult.stop_requests`. Keep existing prompt requests intact. Add action metadata describing whether prompt input is used; Prompt agent uses it, Stop session does not. This is the narrow new OTP-facing operation; no arbitrary product-service access or second imperative stop path in `OtpHost`.
- Create `src-tauri/src/otp_host/session_control.rs` as the narrow cancellation adapter. Inject it into `WorkflowExecutionService` in `active_app.rs`; implement it using existing session repository reads and `AgentSessionApplication::cancel_invocation`. The package imports only `otp_api`.
- Edit `otp_host/workflow.rs` and `workflows/instances.rs`: validate instance/node/session ownership, resolve and persist the exact active invocation ID before requesting cancellation, and record requested/no-op/failed outcomes. If that invocation has already finished, do not cancel a newer one. Duplicate occurrences retain the existing attempt deduplication behavior. Do not hold database writes across runtime cancellation.
- Stop-only actions skip prompt/file materialization. Existing Session Events continue delivering prompts; do not invent a stopped prompt-delivery event or claim cancellation is terminal before runtime updates establish it. Retain attempt outcomes even when no session matches.
- Update `workflows/compiler.rs` and the TypeScript OTP DTOs to recognize both declared action output kinds, with the same eligibility rules for every destination. Update catalogue parity fixtures and test hosts for the added result field.
- Acceptance: package selection and empty-state tests; host integration tests for bound ownership, cancellation invocation ID, already-finished race, failure, deduplication and persisted reopen. Existing prompting/continuation tests remain green. No user session is stopped for verification.

### P4 — Entry parity and observable action results

Outcome: selecting an action works the same for initial entry and connection destinations.

- Retain `entryAction` as a recipe storage field; add default-empty `entryConfiguration` to `workflows/authoring.rs`, `compiled_plan.rs` and corresponding TypeScript contracts. This is storage placement, not another action category. The default preserves current saved recipes; no migration framework or reset.
- In `WorkflowAuthoringScreen.tsx`, mount P2's destination adapter beside the initial destination node setting. Use the same catalogue, details and configuration editor as a connection.
- Edit `workflows/execution.rs`, `execution_transport.rs` and `otp_host/workflow.rs`: pass configured entry parameters, remove the exactly-one-delivery requirement, and return one shared action result containing the attempt ID, zero/multiple prompt event groups and stop outcomes. Update routing receipt wording/counts so a successful stop is not misreported as a delivered prompt.
- Update `src/application/workflowInstances.ts`, `src/infrastructure/workflowInstances/tauriWorkflowInstanceClient.ts` and `WorkflowInstancePanel.tsx`. Display action outcomes; open a delivered session when present. A Stop action can be invoked without dummy prompt text; only prompt-consuming actions show/require prompt content. Display cancellation requested separately from the session's observed terminal status.
- Add default-empty stop request/outcome fields when reading older attempts. Preserve existing demo recipes and records without maintaining parallel legacy implementations.
- Acceptance: save/reload/activate initial and connection destinations using the same action; initial Stop with no session gives a visible no-op; Stop with an active session requests cancellation; Prompt agent still opens delivered sessions. Verify existing multi-session prompting no longer fails solely because entry returned more than one delivery.
- Dependencies: P2/P3 contract convergence. No new graph node type, special entry tool, approval behavior or revision coordination.

## Sequence and validation

Preferred sequence: agree the P1/P3 descriptor and action-result shapes, implement P1, P2, P3, then P4; finish with integrated checks. P1's modal and P3's cancellation adapter can be developed independently after the contract is fixed. P2 shares authoring/profile surfaces with P4, so those edits should be sequential. This identifies overlap, not a request to spawn agents.

Validation placement:

1. Each package supplies the focused tests described above before integration. Use existing Vitest component/client tests and Rust recording-runtime/SQLite seams.
2. Run focused OTP, Workflow, Agent Session integration and affected frontend suites; TypeScript/frontend build, Rust check, and `git diff --check`. Existing full-suite results are not claimed as current evidence.
3. Run a disposable native cancellation exercise: activate a test destination, start a controlled invocation, execute Stop through Workflow, observe terminal cancellation, then prompt the same session again. Confirm other nodes/sessions are unaffected. A fake-runtime pass alone is not live cancellation evidence.
4. Inspect actual rendered picker flows for trigger, destination and both profile contexts. Exercise expansion, details, confirmation, cancellation, keyboard/focus, and narrow layout. Use a synthetic second package only for UI tests; do not import invented production capabilities.
5. Record commands, results and limitations in a slice validation note. No commit, push or integration is implied by the plan.

## Deferrals, risks and current status

No package installation screen, package discovery changes, permission redesign, arbitrary deterministic tool system, extra MCPs, stop-all, archive/delete semantics, or separate entry category. No changes to Harness-derived file authorship or continuation routing scope.

Largest risk is cancellation targeting a later invocation; the pinned-invocation test is required. Other risks are action results being mistaken for prompt deliveries, accidental profile capability expansion, and silently dropping unavailable selections. The specified adapter/integration tests cover those boundaries.

Status: plan complete, implementation not started. Remaining behavioral assumption is the bounded Stop selection policy above. After implementation is requested, use the proposed defaults unless the user revises them; live cancellation and visible UI evidence remain completion gates.
