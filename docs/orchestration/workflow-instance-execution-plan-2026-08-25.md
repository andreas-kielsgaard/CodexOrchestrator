# Workflow Instance Execution Plan

- Date: 2026-08-25
- Revision: 3
- Status: Implemented and validated; feature checkpoint pending
- Candidate branch: `codex/workflow-engine-v1`
- Canonical plan file: `docs/orchestration/workflow-instance-execution-plan-2026-08-25.md`

## Re-ingestion instructions

After context compression or a new implementation turn, read this file in full before acting. Then re-check `git status`, the candidate branch identity, the current candidate and main commits, and relevant source files. The commit and file observations below are a dated baseline, not permanent facts.

This file is the durable plan authority for the approved scope. Record material discoveries in the revision log and update affected sections rather than creating a competing plan.

## Objective

Deliver the smallest happy flow that proves a user can:

1. Define and activate a Workflow type using the existing designer and its start-node marker.
2. Create a named Workflow instance by selecting the Workflow type and an existing repository/branch worktree target.
3. Reach the instance view without creating an Agent Session or modifying the selected worktree.
4. Select the start-marked node and use an in-workflow Agent Session view to submit initial input.
5. Create the Session and first message as one user action, with the Session bound to the Workflow instance, node, and selected worktree.
6. Continue through the existing routing and inspection behavior.

The provider starts when the initial message is submitted, not when the Workflow instance is created. This does not start the Codex desktop application.

## Locked product decisions

These decisions require explicit user approval to change:

- Repository, branch, and worktree attachment occurs at Workflow instance creation, never at definition time.
- The creation dialog collects an activated Workflow type, a required instance name, and a worktree target, then creates the instance and navigates to its view.
- The temporary selector offers only discovered worktrees associated with branches. It presents each choice as repository plus branch; worktree selection is implied.
- The target boundary must be replaceable by the Worktree Review selector without entangling that implementation with Workflow UI, persistence, or execution.
- Creating an instance records AppData only and does not edit the selected worktree.
- No Agent Session exists until the user submits initial input. The Session and first message are created as one user action.
- The start marker only makes a node eligible for the initial human action and makes it selectable before a Session exists.
- There is no start-specific Workflow operation or `startWorkflowInstance` hook. The first message uses the generic Workflow-bound node Session path.
- The in-workflow conversation surface reuses the existing Agent Session controller and complete workspace implementation. It is not a second conversation UI.
- Exactly one start node remains the design target because it matches the current validation and requires the least work. Existing start-node designer UI is retained.
- Task routing and inspection are sufficient for this slice.
- Use the naive happy flow. Do not add generalized fallback, retry, recovery, compatibility, or authority machinery.

## Scope limiters, not enduring design policy

- Workflow-associated Sessions use the instance worktree for now.
- Branch switching is unsupported, but the engine does not police later branch movement.
- Workflow-produced handoff files currently belong in the worktree because execution is rooted there; the engine adds no file-placement policy.
- Workflow runtime data remains alpha/disposable. Migration only needs to preserve the definition data and unrelated AppData required by this feature.
- Ordinary errors may surface through existing Agent Session or dialog behavior. No Workflow-specific recovery UX is required.

## Out of scope

- The Worktree Review selector itself, creating a new worktree, or merging its development branch.
- Multiple-worktree-per-repository-and-branch UX.
- Detached worktrees, cleanliness checks, compatibility checks, branch enforcement, or worktree edits.
- A more general Workflow definition surface.
- New transcript, composer, processing, cancellation, inspection, clipboard, error, or Session-header implementations.
- Start-specific backend semantics, durable launch state, Workflow-specific provider startup, or a generated Workflow instance directory.
- Generalized Session placement policies, handoff-file policy, retry/recovery, or production hardening.
- Live or paid provider validation without separate authorization.

## Dated evidence baseline

At plan approval:

- Candidate worktree: `C:\Users\user\.codex\worktrees\workflow-engine-v1\Codex Orchestrator`
- Candidate branch: `codex/workflow-engine-v1`
- Candidate status: clean
- Candidate commit: `c776122`
- Candidate and then-current main had diverged by 2 and 13 commits respectively.
- `docs/orchestration/` exists and is the established location for durable orchestration plans.
- The current inline Workflow node Session surface already renders `AgentSessionWorkspace`.

Reverify all of these before implementation. Preserve any user-owned dirty or untracked state discovered later.

## Architectural boundaries

### 1. Stable worktree-target contract

Create a React-free application contract at:

- `src/application/worktreeTargets.ts`

It defines a resolved target whose output is stable regardless of selector implementation:

```ts
type ResolvedRepoBranchWorktreeTarget = {
  repository: { id: string; name: string; rootPath: string };
  branch: { id: string; name: string };
  worktree: { id: string; path: string };
};
```

Also define a source interface for listing resolved targets. Stable contracts and consumers must not use temporary naming.

The concrete temporary implementation belongs only in:

- `src/infrastructure/worktreeTargetsTemp/tauriDiscoveredWorktreeTargetSource.ts`
- `src/features/worktreeTargetsTemp/DiscoveredWorktreeTargetSelector.tsx`
- `src-tauri/src/worktree_targets_temp/mod.rs`
- `src-tauri/src/worktree_targets_temp/tests.rs`
- `src-tauri/src/worktree_targets_temp/README-2026-08-25.md`

The dated README should state briefly:

> This module supplies temporary discovered-worktree choices for Workflow instance creation. It will be replaced by the Worktree Review worktree selector when that feature is ready; consumers depend only on the resolved repository/branch/worktree target output.

The temporary source reads the existing AppData repository, branch, and worktree records. It returns only branch-associated worktrees, sorted deterministically across repositories. The temporary UI shows repository plus branch and emits only the resolved stable target.

Product composition injects the temporary selector into Workflow instance creation. The future Worktree Review implementation replaces this binding at composition without changing the dialog, Workflow application contracts, instance model, or execution path.

### 2. Workflow instance model as a contraction

Create frontend instance contracts at:

- `src/application/workflows/instanceContracts.ts`

The model should contain:

- Instance summary: ID, type, name, recipe counts, Session counts, and creation time as supported by current product needs.
- Resolved repository/branch/worktree target.
- Activated recipe projection.
- Associated Sessions.
- Connection activations used by routing evidence.

Do not retain or introduce:

- `startingPrompt`
- instance `workingDirectory`
- `launchStatus`
- mandatory `launchActivation`
- `humanActivations`
- durable unstarted/running/failed launch state

No Sessions means the UI may derive and display “Ready to begin”; it is not a persisted lifecycle state.

Create the Rust instance domain at:

- `src-tauri/src/workflows/instance_domain.rs`

Move instance persistence into:

- `src-tauri/src/workflows/repository/instances.rs`

Keep the existing Workflow repository facade and definition ownership in the existing repository module. Instance creation stores the instance/type/recipe/name, repository facts, branch facts, worktree facts, and timestamp.

Runtime storage becomes:

- `workflow_instances`: instance/type/recipe/name, repository ID/name/root, branch ID/name, worktree ID/path, timestamp.
- `workflow_instance_sessions`: instance ID, node ID, Session ID, association time.
- `workflow_connection_activations`: existing unique routing evidence.

Remove the obsolete prompt, working-directory, launch-activation, and launch-state representation, including `workflow_activations`.

### 3. One generic Workflow node Session path

Create one opaque frontend adapter file:

- `src/application/workflows/workflowAgentSessionClient.ts`

It exports a factory shaped like:

```ts
createWorkflowAgentSessionClient(
  ordinaryClient,
  workflowClient,
  { workflowInstanceId, nodeId },
): AgentSessionClient
```

Private behavior:

- With no selected Session ID, sending a message calls the generic Workflow node-message operation.
- With a selected Session ID, sending delegates to the ordinary Agent Session client.
- Create, list, load, reload, subscriptions, cancellation, disconnect, and other ordinary operations delegate unchanged.

The adapter does not decide start eligibility, calculate targets, own UI state, or expose Workflow concerns through the global Agent Session client contract.

Create one opaque backend coordinator:

- `src-tauri/src/workflows/node_sessions.rs`
- `src-tauri/src/workflows/node_sessions_tests.rs`

Its intention-level internal entry points are:

- `send_human_message(...)`
- `deliver_connection_message(...)`

Both use one private linear delivery path:

1. Resolve the instance, node, recipe, worktree root, and Agent/Harness context.
2. Select or allocate the appropriate Session.
3. Create and bind a Session only when needed.
4. Record the instance/node/Session association.
5. Send through the existing `AgentSessionApplication`.
6. Return the ordinary acknowledgement expected by the existing controller.

The coordinator contains no SQL, Tauri command code, worktree discovery, instance projection, Harness implementation, Agent persistence, start eligibility, or retry/recovery. Do not expose public origin/session-policy enums merely to unify these callers.

The existing Workflow application service becomes a thin facade. Existing connection routing delegates Session delivery to this coordinator and continues to own connection semantics.

### 4. Thin in-workflow Agent Session shell

Extract and expand the existing inline `WorkflowInstanceNodeSessions` composition into:

- `src/features/workflows/WorkflowAgentSessionPane.tsx`
- `src/features/workflows/workflowAgentSessionPane.css`

This is a Workflow-owned host, not an Agent Session view implementation. It owns only:

- Workflow node identity and return/close controls.
- The node’s Workflow-associated Session list.
- Selection of an associated Session.
- Whether the empty new-Session state is available.
- Notification that a newly created Session requires refreshed Workflow associations.

The containing instance view constructs and supplies an already Workflow-bound `AgentSessionClient`. The pane uses `useAgentSession` unchanged and renders `AgentSessionWorkspace` unchanged for both the empty composer and existing Sessions. Existing workspace presentation properties and header-action composition provide guidance, labels, and Workflow return controls.

Pane CSS governs only shell geometry, node navigation, and Session-list layout. It must not restyle or reproduce transcript, composer, processing, error, cancellation, clipboard, inspection, or Session-header internals.

Do not use `SharedAgentSessionPanel`; it assumes an already-existing Session, while this surface must also host the existing workspace’s new-Session state.

Before any Session exists, the instance UI exposes the pane only for the start-marked node. Once created, the same pane and workspace render the Session. The generic backend node-message operation remains valid for any valid node; start eligibility is a UI/design concern only.

### 5. Cohesive instance-creation dialog

Create:

- `src/features/workflows/WorkflowInstanceCreationDialog.tsx`
- `src/features/workflows/workflowInstanceCreationDialog.css`
- `src/features/workflows/WorkflowInstanceCreationDialog.test.tsx`

The dialog receives activated Workflow types, an injected target-selector component, `onSubmit(input)`, and `onClose`. It owns type selection, trimmed required name, resolved target selection, validation, pending state, ordinary creation errors, focus behavior, Escape, and backdrop handling.

`WorkflowScreen` only opens the dialog, supplies dependencies, submits through the Workflow client, and navigates to the new instance. Backend creation remains in the instance boundary; there is no dialog-specific backend file.

### 6. Thin product integration

Expected integration points include:

- `src/features/workflows/WorkflowScreen.tsx`
- `src/App.tsx`
- `src/bootstrap/productApplicationComposition.ts`
- Workflow application contracts and Tauri Workflow client
- `src-tauri/src/workflows/application.rs`
- `src-tauri/src/workflows/transport.rs`
- `src-tauri/src/active_app.rs`
- `src-tauri/src/storage.rs`

`AgentSessionWorkspace`, `useAgentSession`, and `AgentSessionApplication` are reuse anchors and are expected to remain behaviorally unchanged. A discovery may require a small compatible seam adjustment, but any material expansion of their responsibilities requires plan revision and review.

## Deprecation and replacement map

Remove or replace within this slice:

- `launchWorkflowInstance` and `launch_workflow_instance`
- `WorkflowLaunchPreparation`
- `WorkflowLaunchStatus`
- `startingPrompt`
- instance `workingDirectory`
- singleton `launchActivation`
- any proposed `humanActivations`
- `workflow_activations`
- generated Workflow-instance directories
- inline launch dialog
- Workflow-specific launch-failure persistence and presentation
- old combined launch/create code
- old inline node Session host after extraction
- tests whose sole contract is the obsolete launch model

Retain:

- Existing designer and start-node marker.
- Exactly-one-start-node validation.
- Activated recipe projection.
- Direct instance/node/Session association.
- Connection activations and routing evidence.
- Existing routing, MCP, expected-file, and inspection capabilities.
- Existing Agent Session controller and view implementation.

## Storage migration

Advance the active schema from version 43 to 44, subject to revalidation of the live baseline.

The migration may drop and recreate only Workflow runtime tables in dependency order, including connection activations, old activations, Session associations, and instances. Preserve Workflow definitions, roles, nodes, connections, recipes, unrelated AppData, and existing Agent Sessions.

Before dropping the old instance rows, delete only `session_harness_bindings` whose `source_workflow_instance_id` references those disposable Workflow instances. That table has no foreign key to clean these bindings automatically; retaining them would leave stale Workflow authority attached to otherwise preserved Agent Sessions.

No complex conversion of alpha Workflow runtime records is required. Recreate foreign keys and indexes explicitly and verify that obsolete columns and `workflow_activations` are absent.

## Implementation sequence

The sequence is intentional but adaptable when discoveries reveal a safer dependency order.

### ARCH-0 — Refresh the candidate baseline

- Reverify clean/dirty state and branch identities.
- Merge current main into `codex/workflow-engine-v1`; do not rebase.
- Resolve only feature-relevant conflicts and preserve unrelated state.
- Run a focused baseline check before feature edits.

Gate: exact merged candidate commit and baseline failures, if any, are recorded separately from feature defects.

### ARCH-1 — Establish the replaceable target boundary

- Add the stable target contract and selector interface.
- Add the isolated temporary Rust discovery module and dated README.
- Add the temporary infrastructure source and selector component.
- Bind the temporary implementation only in product composition.
- Prove deterministic, read-only, branch-associated target discovery.

Gate: a fake replacement selector can feed the creation dialog without importing any temporary implementation.

### ARCH-2 — Replace the Workflow instance model and persistence

- Add the narrowed frontend and Rust instance domains.
- Split instance persistence into the repository child module.
- Replace instance creation with AppData-only target registration.
- Simplify Session association and retain connection routing evidence.
- Add the version-44 runtime-table migration.

Gate: creating an instance writes no Session, provider invocation, generated directory, or selected-worktree file.

### ARCH-3 — Add the generic Workflow node Session delivery boundary

- Add the opaque frontend Agent Session client adapter.
- Add the backend node Session coordinator.
- Route both human messages and connection messages through the shared private lifecycle.
- Reduce the Workflow application/transport layers to intention-level delegation.

Gate: a first human send atomically yields the persisted Session association and ordinary first message, and later sends use the ordinary Agent Session path.

### ARCH-4 — Compose the dialog and in-workflow Session pane

- Extract the inline Session host into the thin Workflow pane.
- Reuse `useAgentSession` and `AgentSessionWorkspace` unchanged.
- Replace the inline launch dialog with the injected-target creation dialog.
- Update the instance view so only a start-marked node can open the empty composer before a Session exists.
- Navigate to the instance immediately after successful creation.

Gate: there is one Agent Session view implementation, and the full happy flow is reachable without launch-specific code.

### ARCH-5 — Converge routing, cleanup, and validation

- Update connection routing to derive `worktree_root` from the instance target.
- Remove obsolete launch model, schema, UI, and tests.
- Preserve existing routing, fanout, MCP, expected-file, and inspection behavior.
- Run focused and production validation, then inspect the narrow UI manually or through available local UI tooling.

Gate: all acceptance evidence below is collected, with residuals stated explicitly.

## Test plan

### Temporary target boundary

- Returns only worktrees associated with branches.
- Emits exact repository, branch, and worktree identity/path facts.
- Sorts deterministically across multiple repositories.
- Performs no writes or worktree mutation.
- Displays repository plus branch, with worktree identity implicit.
- Selector emits the exact stable target output.
- A fake injected selector replaces the temporary selector without changing consumers.

### Instance domain, persistence, and migration

- Creating an instance persists the resolved target and creates no Session or provider invocation.
- Instance creation does not write the selected worktree.
- Obsolete prompt, working-directory, and launch-state columns are absent.
- `workflow_activations` is absent.
- Session association directly records instance, node, and Session.
- Initial input exists only in Agent Session history.
- New Session execution directory equals the instance target worktree path.
- Migration resets only Workflow runtime data while preserving definitions and unrelated AppData.

### Frontend Workflow Agent Session client adapter

- First send is intercepted with the exact instance/node context and returns the expected acknowledgement.
- Sends with an existing Session ID delegate to the ordinary client.
- Create, list, load, reload, subscriptions, cancel, and disconnect delegate unchanged.
- The adapter performs no eligibility check or target calculation.

### Backend node Session coordinator

- Human delivery creates a Session and first invocation, records its association, and derives worktree/Harness context.
- A valid non-start node works at the application layer, confirming start eligibility is not embedded there.
- Connection delivery uses the same lifecycle.
- Existing receiver Session reuse remains correct.
- Fanout, connection independence, and routing lanes remain correct.
- Ordinary downstream errors surface without Workflow-specific fallback state.

### In-workflow Agent Session pane

- An empty eligible node renders the existing workspace’s new-Session composer.
- An existing Session renders through `AgentSessionWorkspace`.
- Selecting another associated Session updates the selected controller Session.
- First send uses the supplied Workflow-bound client.
- Creation notification refreshes Workflow associations.
- Workflow return and node-selection controls work.
- The Workflow feature contains no custom transcript, composer, processing, or error implementation.
- Existing `AgentSessionWorkspace` and controller tests remain unchanged and pass as reuse evidence.

### Creation dialog and screen integration

- Only activated Workflow types are selectable.
- Name is required and trimmed.
- A resolved target is required.
- Submission forwards the exact type, name, and target once.
- Pending state prevents duplicate submission.
- Ordinary rejection is displayed without durable launch state.
- Focus, Escape, backdrop, labeling, and dialog semantics remain accessible.
- `WorkflowScreen` tests cover only opening, dependency supply, submit, and navigation integration.

### Obsolete tests to remove or rewrite

Remove assertions that require:

- Initial prompt on the Workflow instance.
- Persistent launch status.
- Singleton launch activation.
- A failed Workflow-instance lifecycle state.
- Workflow-specific Harness binding failure state.
- Launch acceptance as a separate operation.

Preserve and update routing tests for receiver reuse, fanout, connection independence, MCP, expected files, Session inspection, and activated-recipe behavior.

### Validation commands and evidence

Discover exact project scripts after the baseline merge, then run the narrowest supported checks for:

- Target boundary frontend and Rust tests.
- Workflow frontend tests.
- Workflow Rust tests.
- Relevant Agent Session tests.
- Storage migration tests.
- Production frontend build.
- Narrow-viewport and focusable-control inspection of the dialog and Workflow view.

Use deterministic fake/injected providers only. Record pre-existing or dependency failures separately. Do not infer end-to-end success from a build alone.

## Acceptance evidence map

| Objective                           | Required evidence                                                                          |
| ----------------------------------- | ------------------------------------------------------------------------------------------ |
| Replaceable target selection        | Stable target contract, isolated temp modules and README, injected fake-selector test      |
| Instance registration only          | Persistence test showing target saved with no Session/provider/worktree write              |
| First human action                  | Coordinator test showing Session association and first message from one submitted action   |
| Existing Agent Session reuse        | Pane composition tests plus unchanged workspace/controller test suite                      |
| Correct worktree execution          | Session/application test proving execution root equals stored target worktree path         |
| Existing engine capability retained | Routing, fanout, MCP, expected-file, and inspection regression tests                       |
| Usable happy flow                   | Dialog/screen integration test and focused UI inspection                                   |
| Safe alpha migration                | Migration test preserving definitions and unrelated AppData while resetting runtime tables |

## Risks and controls

- **Baseline drift:** reverify source and schema after merging main; update this plan if ownership materially changed.
- **Accidental second conversation UI:** keep Workflow pane tests about composition, and treat unchanged Agent Session tests as reuse evidence.
- **Temporary selector leakage:** stable consumers import only the target contract/selector interface; concrete binding occurs in composition.
- **Launch-model residue:** search frontend, Rust, schema, and tests for all deprecated symbols before acceptance.
- **Over-centralized coordinator:** keep SQL, transport, projection, discovery, eligibility, and policy outside `node_sessions.rs`.
- **False atomicity claim:** prove the user-visible single action and persisted outcomes precisely; do not claim a database transaction unless implementation evidence supports it.
- **Dirty worktree damage:** stop before overlapping user changes; never clean, reset, or overwrite them.
- **Scope expansion from discoveries:** use the change protocol below.

## Discovery and change protocol

Implementation is expected to refine this plan as current code is inspected.

The implementer may, while preserving the approved boundaries:

- Adjust private helper names, file-internal structure, exact test placement, or step order.
- Reuse a newly discovered existing contract or helper instead of adding a duplicate.
- Make small compatible seam changes required to extract existing composition.
- Add focused tests that become necessary to prove the same acceptance criteria.

For each material discovery, append a revision-log entry containing the date, evidence, affected decision, and plan impact. Update the relevant section so this file remains sufficient for re-ingestion.

Stop and request user direction before:

- Changing a locked product decision.
- Broadening the definition surface or target-selection capability.
- Adding a durable lifecycle, retry/recovery system, or engine file policy.
- Changing the public target boundary so the future selector cannot replace it cleanly.
- Reimplementing Agent Session UI/controller behavior.
- Merging the Worktree Review development branch.
- Performing destructive cleanup or overwriting user-owned changes.
- Using a live/paid provider.

When a discovery affects only an adaptable implementation detail, revise this document and continue once implementation is authorized. Do not silently preserve obsolete architecture merely because it appeared in this initial file map.

## Launch register

| Step                                  | Status   | Dependencies           | Authorization/evidence needed                                            |
| ------------------------------------- | -------- | ---------------------- | ------------------------------------------------------------------------ |
| ARCH-0 Refresh candidate baseline     | Complete | Approved plan          | Main merged at `6b15186`; lockfile reconciled at `9cb9783`               |
| ARCH-1 Replaceable target boundary    | Complete | ARCH-0                 | Stable contract and isolated temporary source/selector accepted          |
| ARCH-2 Instance model and persistence | Complete | ARCH-1                 | Target snapshot, direct Session association, and v44 reset verified      |
| ARCH-3 Generic node Session path      | Complete | ARCH-2                 | Human and routed deliveries pass through the shared private lifecycle    |
| ARCH-4 Pane and creation dialog       | Complete | ARCH-1, ARCH-2, ARCH-3 | Dialog, navigation, adapter, and reused Agent Session workspace verified |
| ARCH-5 Convergence and acceptance     | Complete | ARCH-2, ARCH-3, ARCH-4 | Scoped suites green; broader residual documented below                   |

Implementation was authorized on 2026-08-25. All approved architecture steps are complete in the candidate worktree; the implementation commit is the remaining checkpoint action.

## Revision log

### Revision 3 — 2026-08-25

- Added the stable React-free target contract, isolated temporary frontend and Rust target modules, dated replacement README, and the sole temporary product-composition binding. The Rust source opens the legacy AppData database read-only and returns only branch-associated worktrees in deterministic order.
- Replaced the Workflow launch model with target-only instance registration, direct instance/node/Session association, a v44 alpha runtime reset, and no `workflow_activations` table. Instance creation is proven not to create the selected worktree, a Session, or a provider invocation.
- Added the generic Workflow node-message transport and the private `node_sessions.rs` delivery coordinator. The existing application test fixture covers first-human delivery, direct human delivery to a valid non-start node at the application boundary, receiver reuse, fanout, MCP, expected files, and activated-recipe behavior; a separate `node_sessions_tests.rs` was not added because it would duplicate that large fixture without adding a new boundary proof.
- Extracted `WorkflowAgentSessionPane` as a Workflow-owned shell around unchanged `useAgentSession` and `AgentSessionWorkspace`. Added the Workflow-bound Agent Session client adapter, creation dialog, injected target selector, start-node empty-composer eligibility, derived “Ready to begin” presentation, and immediate typed navigation after creation. Removed obsolete launch UI and the dead inline Session-host CSS.
- Current validation evidence: 59 focused frontend tests passed across the target boundary, dialog, adapter, pane, screen, App navigation, Tauri client, persistence coordinator, and unchanged Agent Session workspace/controller; TypeScript, ESLint with no errors, and the production frontend build passed. Focused Rust evidence is 33 Workflow tests, 27 storage tests, and 3 temporary target tests, all passing.
- The full non-live Rust library run produced 604 passes, 2 ignored tests, and one local MCP body timeout under parallel load. The exact timed-out test passed when rerun alone, so there is no reproducible residual failure attributable to this change.
- The v44 migration initially exposed three historical partial-schema tests whose databases lacked the Agent Session parent table. The reset now deletes stale Workflow Harness bindings only when Agent Session storage is present, preserving the intended real-database cleanup while retaining compatibility with those existing migration proofs.
- A native candidate-window viewport pass was not performed: the only running Codex Orchestrator window belonged to the canonical checkout, not this candidate. Dialog focus, Escape, backdrop, labeling, and narrow CSS behavior remain covered by focused component tests and static review; no live provider was used.

### Revision 2 — 2026-08-25

- Merged current main into the candidate at merge commit `6b15186`.
- Resolved the sole content conflict by retaining the feature branch's runtime `reqwest` dependency and main's `live-tests` feature gate and `test-fast` profile.
- The first focused Rust run showed that the merge result's slim lockfile did not represent the retained runtime features. Cargo regenerated the required Rustls/AWS-LC dependency entries; this is an ARCH-0 lockfile reconciliation, not Workflow feature scope.
- Post-merge baseline evidence: 25 focused frontend Workflow tests passed and 36 focused Rust Workflow tests passed.
- Current-source inspection corrected the composition path to `src/bootstrap/productApplicationComposition.ts` and confirmed that `AgentSessionClient` has no archive method. Adapter scope now names only the actual client contract rather than adding an unused API.
- Backend inspection found that discovered repository/branch/worktree facts live in the legacy AppData database while Workflow runtime data lives in the active database. The temporary target source therefore opens the legacy database read-only and persists target facts as snapshots without cross-database foreign keys.
- Backend inspection also found that `session_harness_bindings` has no foreign key to Workflow instances. The v44 reset now explicitly removes only bindings for the disposable old Workflow instances before dropping runtime rows, while retaining their Agent Sessions and history.

### Revision 1 — 2026-08-25

- Converted the approved discussion into a durable execution plan.
- Incorporated the adversarial reviews as constraints on the instance model, opaque frontend/backend Session boundaries, and reuse of the existing Agent Session workspace.
- Named the temporary Rust folder `worktree_targets_temp` and required its dated replacement README.
- Added explicit adaptability rules so discoveries can refine implementation without silently changing approved product scope.
