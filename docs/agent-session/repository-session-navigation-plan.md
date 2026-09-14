# Repository-based Agent Sessions

Status: implemented on 2026-09-14 in `codex/agent-sessions-navigation-refinement`, based on local main `e2bfc6c`. Verification results are recorded below.

Baseline: local `main` at `e2bfc6c`. The existing edit to this plan is carried forward; the separate `docs/architecture/legacy-task-retirement-plan.md` is outside this work. This replaces the implementation approaches recorded on September 8 and 11.

## Functional target

Every registered repository has a folder, including repositories with no sessions. Each contains collapsible **Workflows** and **Sessions** sections. Workflows contains a folder for each instance targeting that repository.

- Ordinary sessions belong to Unfiled unless explicitly created in or moved to a folder. A working directory alone never implies placement.
- Workflow-owned sessions default to their owning instance. Moving any session changes display placement only; workspace, workflow ownership, profile, harness, identity, and runtime binding remain unchanged.
- Inside an instance folder, sessions not owned by that instance appear first under **Added sessions**; its own sessions follow under **Workflow sessions**. A session from another workflow retains its original-owner marker. Moving an owned session back to its owning instance places it among that instance's owned sessions.
- Repository and grouping-header creation actions start an ordinary repo session. An instance's action starts an ordinary session placed in that instance. Both use the containing repository's main working tree. They create no workflow address.
- Folder actions appear on hover and keyboard focus. Global/Unfiled creation retains the current allocated-empty-workspace behavior when no directory is supplied.
- Support drag/drop and a right-click/keyboard menu with **Pin/Unpin**, **Move to**, and **Copy deeplink**.
- Pins are shortcuts above the repository tree; the original folder entry remains. Sidebar pinning is separate from immutable Session Profile pinning.

“Messages” means sidebar session rows. Initially show five rows per session-containing folder and in Unfiled; Pinned shows all. An instance has one five-row budget across Added sessions followed by Workflow sessions. Labels, repository headers, and instance-folder lists do not count.

**Show more** reveals the remaining rows in that folder. Disclosure stays in UI memory; no server pagination or saved disclosure settings. Session groups sort by activity time descending with session ID as tie-breaker; pins sort newest first by pin time. Moving/pinning does not modify activity timestamps. Opening a session through a deeplink expands its folder and reveals its row.

## Current evidence and resulting decisions

| Current code                                                                                                                                                                                     | Decision                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `repository_catalog/application.rs` already exposes persisted `list_registered()` and `resolve_verified()`. Workflow targets and Worktree Review share this catalog.                             | Reuse registration; no branch integration, second catalog, or discovery adapter.                             |
| Active storage is schema 48 in `codex-orchestrator-active-v3.sqlite`. Current recipe instances and session addresses share this database. The old `workflow_instance_sessions` table is retired. | Add organization metadata to active storage; no V1 reconstruction, old-ID mapping, or cross-database bridge. |
| `agentSessionNavigation.ts` and `SessionSelector.tsx` still build/render the Epic hierarchy.                                                                                                     | Replace that hierarchy and remove its inputs and related-product controls.                                   |
| `useAgentSessionCollection.ts` exists, but `useAgentSessionController.ts` retains duplicate collection state, `skipCollection`, and types derived from the combined controller.                  | Complete the split; do not add a competing collection hook.                                                  |
| Standalone Agent Sessions and `ProfiledSessionPane.tsx` repeat profile loading, execution selection, delivery loading, and identity-update wiring.                                               | Share that conversation behavior across both current consumers. Folder state remains outside it.             |
| First send uses `AgentSessionProfileClient.startDirectUserSession`. The hook's execution branch precedes its `startSession` fallback.                                                            | Extend the profiled boundary; a generic folder-client wrapper would bypass production behavior.              |
| `creation.rs` already prepares sessions; ordinary and addressed inserts already share `repository/mapping.rs::insert_session`.                                                                   | Reuse both. Add initial placement beside that insert, not a second session persistence implementation.       |

Changes since `ac22f01` principally concern Worktree Review branch/commit selection and retained checkouts. They do not replace these session boundaries.

## Ownership after implementation

The navigation feature is product composition: it reads repository, workflow, and session facts. It must not make core session execution depend on the workflow engine.

```text
SessionSelector / SessionTree
    <- pure frontend navigation projection
    <- useAgentSessionCollection
    <- SessionNavigationClient
    <- session_navigation::SessionNavigationService
         -> RepositoryCatalog
         -> WorkflowInstanceStore + workflow ownership projection
         -> SqliteAgentSessionRepository (summaries, addresses, organization)

Folder first send
    -> same profiled start command
    -> SessionNavigationService resolves folder and main directory
    -> AgentSessionApplication prepares, persists, and sends
         -> shared session insert + optional initial placement
```

| Responsibility                                   | Owner                                                                         | Consumers and limits                                                                                      |
| ------------------------------------------------ | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Registered repositories and main-tree resolution | Existing `repository_catalog` / `repository_context`                          | Navigation and folder creation reuse them; Worktree Review retains its own checkout-selection behavior.   |
| Workflow instance/node ownership                 | `workflows/session_navigation.rs` with existing `address_references.rs`       | Supplies typed ownership facts. The sidebar never interprets address namespace strings.                   |
| Placement and sidebar pins                       | `agent_sessions/organization.rs` and `repository/organization.rs`             | Navigation mutations and initial session persistence; no execution effects.                               |
| Combining facts and resolving folder targets     | New `session_navigation/` product module                                      | Thin Tauri commands consume it. No policy or SQL in `active_app.rs`.                                      |
| Grouping, order, and row identity                | `application/agentSessions/navigation.ts`                                     | Sidebar, Move to destinations, and selection reveal use the same model.                                   |
| Expansion, Show more, focus, menu, and drag UI   | `SessionTree.tsx`, `useSessionTree.ts`, `SessionContextMenu.tsx`              | Feature-local behavior; no generic tree framework.                                                        |
| Conversation and profiled message execution      | `useAgentSession.ts`, `useProfiledAgentSession.ts`, existing Rust application | Standalone and embedded conversations; no collection queries or placement mutation in conversation hooks. |

This replaces the earlier proposal for an all-purpose `agent_sessions/organization/` coordinator/repository. The product composition belongs above the participating modules; session-owned metadata belongs beside existing session persistence.

## Backend contracts and flow

### Navigation facts

Add one `load_agent_session_navigation` query returning existing session summaries plus small repository, instance, ownership, and organization records. Return facts, not rendered tree rows.

- Read registered repositories with `list_registered()`; do not call discovery or the Worktree Review overview.
- Add a lightweight instance read to `WorkflowInstanceStore`: instance ID/name, repository ID, and the node IDs/names needed for ownership labels. Select these from current recipe JSON; do not load attempts or use per-instance detail endpoints.
- Add a bulk logical-address read beside `load_address` in `agent_sessions/repository/addressing.rs`. It returns opaque session/address pairs.
- The workflow-owned projection recognizes `workflow/instance/<id>` scopes and `workflow/node/<id>` subjects using the existing typed-reference semantics and matching instance/node facts. Other namespaces do not imply workflow ownership.
- Reuse the existing summary projection, including pending-request attention. It currently reads active invocation events to calculate that attention; this feature adds no parallel transcript/history query or summary optimization project.

`SessionNavigationService` composes these reads. Keep each SQL query with the module owning its records; do not introduce a cross-module SQL repository or a second durable navigation snapshot.

### Organization storage

Add `agent_session_organization`:

| Field                 | Meaning                                                          |
| --------------------- | ---------------------------------------------------------------- |
| `session_id`          | Primary key and foreign key to the session                       |
| `placement_kind`      | `default`, `unfiled`, `repository`, or `workflow_instance`       |
| `placement_target_id` | ID for repository/instance placement; absent for default/unfiled |
| `pinned_at`           | Nullable sidebar pin timestamp                                   |

Use a constraint for placement-kind/target pairing. Missing row means default placement and unpinned. Default placement resolves to the recorded workflow owner, otherwise Unfiled. Explicit Unfiled overrides workflow default placement. Pinning a default-placed session preserves that default.

The frontend resolves display placement once in the pure projector. A row's ownership marker comes from ownership facts independently of its folder or pin. No backfill based on directories, profiles, or old Epic references.

Use `SqliteAgentSessionRepository` with methods implemented in `repository/organization.rs` for reads, move/pin, and atomic creation with initial placement. Reuse the existing `insert_session` and validation helpers. Add only the atomic creation operation needed by `AgentSessionApplication` to its existing repository port; update the two current test implementations directly. No second repository class or generic transaction callbacks passed into application code.

### Main working tree

Add `RepositoryCatalog::resolve_main_working_tree(repository_id)`, backed by an explicit method on the existing `WorktreeInventoryReader`. Keep raw Git interpretation there.

Git documents that the main worktree is first in the unfiltered `worktree list` output. Use that fact before catalog sorting or filtering, and return its available canonical directory. This does not mean the first UI worktree target or a branch named `main`. See [Git worktree list](https://git-scm.com/docs/git-worktree#Documentation/git-worktree.txt-list).

The catalog verifies repository identity and exposes this directory to folder creation. Its current `anchor_root` is an available checkout and is not a main-tree guarantee. No new persisted main-path field, registration redesign, or worktree switching is needed.

### Profiled creation

Keep the existing `start_direct_user_agent_session` command name and `AgentSessionProfileClient.startDirectUserSession`. Introduce a named first-send input rather than continuing to extend an inline `Omit` type. Add optional `folderTarget`; existing-session messages have no folder input.

Move this start command's thin transport implementation to `session_navigation/transport.rs`. Leave pinned-profile reads and existing-session sends in `agent_sessions/transport/selections.rs`.

1. The product service resolves an optional repo/instance folder. For a folder start, its main tree supplies the working directory and its target supplies initial placement. With no target, preserve the current directory/allocated-workspace behavior.
2. Delegate to `AgentSessionApplication`. Extract the direct-user start/send sequence into `application/direct_user.rs`; keep default/profile resolution and runtime conversion in `configuration.rs`.
3. Resolve the default Capability Profile and runtime profile at that directory. Validate the first message and its runtime choices before session persistence.
4. Reuse `prepare_session_with_id` for workspace preparation and pinned session construction.
5. Persist the session and optional initial placement in one active-database write. Ordinary creation and addressed creation retain their existing shared insert.
6. Commit, then use the existing resolved direct-user send path.

The application receives a resolved directory and session-owned placement value, never a catalog or workflow executor. Filesystem preparation and runtime launch stay outside database writes. Folder starts never use Session Event addressed-session creation.

## Frontend state and consumption

### Selection has one owner

Replace the overloaded selected-ID/null input for the standalone view with:

```ts
type SessionNavigationSelection =
  | { kind: 'initial' }
  | { kind: 'session'; sessionId: string }
  | { kind: 'draft'; draftId: string; folderTarget: SessionFolderTarget | null };
```

The existing product-navigation state owns this value in the running app. The standalone test/development host supplies local state. `useAgentSessionCollection` owns fetched facts, loading, refresh, and organization actions; it no longer keeps a second selected-session state or decides that null means “select the first row.”

The screen resolves initial selection after loading; a draft remains a draft across refresh. Starting another draft creates a fresh draft ID even in the same folder. Draft text stays in the conversation hook and is not persisted into routing/history. Restoring navigation restores existing-session destinations; unsent drafts are local to the current view lifetime.

Pass a draft identity to `useAgentSession` so composer reset and first-send completion are associated with the correct draft. Pass the folder target only through first-send context. A successful creation selects its session only if that draft is still current. Folder identity never becomes a React key for an existing conversation; moving/pinning cannot remount or redirect it.

### Complete the collection/conversation split

Move the controlled conversation implementation into `useAgentSession.ts`, declare its workspace-controller interface directly, and delete `useAgentSessionController.ts` once callers/tests migrate. Remove its collection state, list subscriptions, summary reloads, selection commands, and `skipCollection` branch. Retain steering, approvals/questions, transcript loading, workspace resolution, per-message choices, and existing generation checks.

Extract `useProfiledAgentSession.ts` from the repeated wiring in `AgentSessionScreen.tsx` and `ProfiledSessionPane.tsx`. It composes the existing execution-selection hook, conversation hook, delivery query, identity update, and send-unavailable reason. Both consumers adopt it. Each keeps its own presentation/wrappers; do not create a configurable universal conversation component.

`WorkflowInstanceView` continues consuming `ProfiledSessionPane`. Existing managed/embedded consumers in EpicPlanBuilder, SharedAgentSessionPanel, ProductiveProductDecisionsPanel, and HarnessInspectorDevelopmentSurface import the extracted conversation hook directly. Preserve their current client/send behavior; no folder controls or new profile policy are introduced into those hosts.

### Tree behavior

Move the replacement projector into `src/application/agentSessions/navigation.ts`; remove the old root-level module after moving still-used orchestration-origin types to their owner.

Keep `SessionSelector` as the sidebar shell. `SessionTree` renders rows and exposes folder create/drop actions. `useSessionTree` owns view state and derives the exact visible-row sequence used by rendering and keyboard movement, including Show more. Keep pure visibility helpers with this feature rather than in the backend or projector.

Use distinct row IDs for pinned shortcuts and folder entries referring to the same session. A fresh open/reveal action finds the placed row and expands it; later metadata refreshes do not repeatedly reopen manually collapsed folders.

Drag/drop and Move to call the same move operation. Repository and grouping headers target the repo's Sessions placement; instance headers target that instance; Unfiled clears explicit folder placement. Menu pin state and ownership labels come from the same model. Use sibling controls for folder expansion and creation, avoiding nested buttons.

### Refresh

Retain the collection's generation guards, debounced summary-update handling, and `sessionAttention.ts`. Route initial load, explicit refresh, successful move/pin/create, and existing `workflow-instance-updated` notifications through one reload function. Reload on entering the view so newly registered repositories/empty instances appear.

The workflow notifier currently reports execution records, not instance creation. Do not claim it supplies a complete catalog feed or add a generalized invalidation bus for this single-window happy flow.

## Concrete file changes

Paths below are repository-relative. Unqualified frontend filenames in this table are under `src/features/agentSessions/`.

| Action         | Files                                                                                                                                                         | Result                                                                                                |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Create         | `src-tauri/src/session_navigation/{mod,application,transport}.rs`                                                                                             | Product service, fact DTOs, load/move/pin commands, and relocated first-send command.                 |
| Create         | `src-tauri/src/agent_sessions/organization.rs`, `repository/organization.rs`                                                                                  | Placement/pin values, schema, and methods on the existing repository.                                 |
| Create/adapt   | `src-tauri/src/workflows/session_navigation.rs`, `instances.rs`, `address_references.rs`                                                                      | Lightweight instance/node projection and workflow interpretation of address facts.                    |
| Adapt          | `src-tauri/src/agent_sessions/repository/{mod,addressing}.rs`, `ports/repository.rs`                                                                          | Bulk address read and atomic initial-placement operation; reuse mapping helpers.                      |
| Extract/adapt  | `src-tauri/src/agent_sessions/application/{direct_user,configuration,creation,mod}.rs`, `transport/selections.rs`                                             | Shared profiled start/send sequence; no duplicate creation or default resolution.                     |
| Adapt          | `src-tauri/src/repository_catalog/application.rs`, `repository_context/worktrees.rs`                                                                          | Explicit main-tree resolver through the current Git reader.                                           |
| Adapt          | `src-tauri/src/storage.rs`                                                                                                                                    | Organization migration and fresh initialization; recheck version 48 at implementation time.           |
| Create         | `src/application/agentSessions/organization.ts`, `navigation.ts`                                                                                              | Typed folder/organization facts, navigation client, pure projector, and explicit selection type.      |
| Create/adapt   | `src/infrastructure/agentSessions/tauriSessionNavigationClient.ts`, `tauriAgentSessionClient.ts`, `src/application/agentSessions/selections.ts`               | Navigation transport and named folder-aware profiled first-send input.                                |
| Adapt          | `src/features/agentSessions/useAgentSessionCollection.ts`                                                                                                     | Sole navigation-data owner with one refresh path; own its types directly.                             |
| Extract/delete | `useAgentSession.ts`, `useProfiledAgentSession.ts`, `useAgentSessionController.ts`                                                                            | Separate conversation behavior, share profiled wiring, remove combined mode.                          |
| Adapt          | `AgentSessionScreen.tsx`, `ProfiledSessionPane.tsx`, feature barrel and listed embedded callers                                                               | New sidebar composition; shared conversation behavior; direct imports.                                |
| Extract        | `SessionTree.tsx`, `useSessionTree.ts`, `SessionContextMenu.tsx`, `sessionNavigation.css`                                                                     | Focused tree interactions and styles taken from SessionSelector/agentSession.css.                     |
| Move/delete    | `src/application/agentSessionNavigation.ts` -> `agentSessions/navigation.ts` and `orchestrations/navigation.ts`                                               | New tree model and retained product-origin types have separate owners; old Epic traversal disappears. |
| Adapt          | `src/application/productNavigation.ts`, `src/app/App.tsx`, `ProductCommandBar.tsx`, orchestration type consumers                                              | Explicit session selection, generic session/invocation focus, and direct type imports.                |
| Adapt          | `src-tauri/src/active_app.rs`, `active_app/sessions.rs`, `src/bootstrap/productApplicationComposition.ts`                                                     | Wire existing services into navigation and expose its client. Only composition here.                  |
| Create/adapt   | `src/application/agentSessions/deepLinks.ts`, `src/infrastructure/agentSessions/tauriSessionDeepLinks.ts`, Tauri config/capabilities and dependency manifests | Format/parse local session links; receive them through the plugin and existing product navigation.    |

Remove Epic/Sprint traversal, semantic-role grouping, planning-draft folders, obsolete alphabetical session sorting, sidebar Related product views, and the Epic return banner. Update their tests and styles. Keep the existing application's separately owned contextual navigation where still consumed; no compatibility adapter in the new sidebar.

Do not recreate the old WorkflowScreen, V1 storage, folder-client wrapper, catalog facade, or new collection hook. No generic tree library, repository framework, or full application/router refactor. The separate legacy task retirement can proceed independently; this plan neither implements nor requires it. Respect `src-tauri/AGENTS.md`: active behavior stays out of quarantined `lib.rs`.

## Deeplinks and native integration

Use `codex-orchestrator://sessions/<session-id>` for application session identity. Reuse `sessionClipboard.ts` for copying. Parse into the existing typed product-navigation destination, then select/reveal; opening a link sends no messages.

Use the Tauri deep-link plugin's initial URL read and running-app listener, with Windows single-instance forwarding. Put plugin setup in current application composition. The JavaScript plugin adapter provides reception, so do not also create a custom Rust session-link event bus/parser. See [Tauri deep linking](https://v2.tauri.app/plugin/deep-linking/).

Update `package.json`/lockfile, `src-tauri/Cargo.toml`/lockfile, `tauri.conf.json`, and the relevant capability permissions. Configure the webview for native HTML5 dragging using [Tauri's drag/drop setting](https://v2.tauri.app/reference/config/#dragdropenabled). Keep cross-machine/multi-installation routing and deleted development-runtime scripts outside scope.

## Delivery and verification

1. Add session organization values/storage and catalog main-tree resolution. Prove session+placement rollback together, move/pin preservation, and reopen persistence.
2. Add the lightweight workflow facts and product navigation service. Extend profiled first send through the shared direct-user sequence; verify both ordinary and folder starts.
3. Replace the projector/tree and make selection explicit. Complete the collection/conversation split and migrate both profiled consumers. Remove obsolete hierarchy code in the same change.
4. Add local link entry, native drag/drop configuration, and the focused UI acceptance flow. Update the current Agent Sessions documentation.

Focused acceptance:

- Two registered repositories, one with no sessions, and current recipe instances group correctly. Existing ordinary sessions stay Unfiled.
- Six sessions show five; Show more expands only that folder. Six pinned shortcuts all show. Mixed Added/Workflow rows share one limit and preserve their ordering.
- Register from a linked checkout, then create from repo and instance folders: both sessions use the main tree and resolve defaults there; neither gets a workflow address. Global creation retains allocated workspaces.
- Move ordinary and workflow-owned sessions among repos, instances, and Unfiled; pin/unpin and reopen. Cwd, workspace origin, runtime binding, profile, harness, identity, address, activity timestamps, and workflow node listings remain unchanged.
- Refresh an unsent folder draft; start another draft with a different target; first send uses the current target. Placement/pin changes do not reset an active conversation.
- Retain steering, pending approvals/questions, unselected attention updates, profile choices, embedded send behavior, and selected-session load guards in their new owning tests.
- Copy/open a session beyond the initial five in cold and running Windows app flows. Verify real native dragging and keyboard-accessible folder actions, menu, and disclosure.

Test behavior at its owning boundary: pure placement/visibility tests; database migration/transaction tests; profiled start tests; collection/conversation tests; and a recorded UI flow. Do not add tests merely because a wrapper/file exists. Run the relevant Rust/TypeScript suites and frontend build after implementation. Native protocol registration and dragging need an actual desktop check; unit tests cannot establish them.

Scope is the agreed local happy flow: no custom folders, folder pinning, bulk actions, arbitrary reordering, search, server pagination, discovery redesign, workflow lifecycle changes, or old Workflow V1 recovery.

## Implementation result

The module ownership and deletions above are implemented. Session selection now has one owner in product navigation; collection refresh and organization writes do not select sessions. Shared conversation/profile hooks serve standalone and workflow panes. The retired Epic sidebar projector, combined conversation/collection controller, related-product banner, and their obsolete hierarchy tests are removed. Retained orchestration destination types live in `application/orchestrations/navigation.ts`.

Storage schema 49 adds only session placement and pin metadata. Workflow ownership still comes from typed logical addresses. First send resolves folder context through the current repository catalog, then uses the existing profiled session preparation and shared insert in one transaction.

Validation on the implementation worktree:

- All 980 frontend tests (174 files), the frontend production build, and focused lint pass. Existing Vite chunk-size and Rust dead-code warnings remain.
- Rust: 70 Agent Session tests, 40 storage tests, five repository catalog tests, and the workflow ownership projection test pass. The folder-creation integration test uses a real Git repository registered from a linked checkout and verifies main-tree cwd, pinned profile, no workflow address, lightweight workflow facts, and unchanged execution state after organization writes.
- UI tests cover folder drafts across refresh, moves/pins preserving composer state, added/owned ordering, shared five-row disclosure, all pin shortcuts, context actions, drag routing, profile choices, steering, approvals, and native-link reception.
- A disposable Windows application database verified the repository tree, empty repository, workflow owner label, five-row default, Show more, keyboard Pin, and cold/running-instance deeplink selection. The warm-link observation prompted scrolling the selected row into view.
- Native drag automation did not establish a successful drop. Drag event routing is covered in UI tests; an interactive Windows drag check remains. Protocol activation was exercised by passing the URL to the executable, not by installing the bundle and invoking the registered Windows URL handler. No live provider turn was sent during this navigation smoke test.

The feature adds no compatibility path for retired Epic/Sprint ownership and does not implement the separate legacy-task retirement plan.
## Agent command access

The requested agent interface is implemented in `session_navigation/agent_access.rs`, `application/agentSessions/agentAccess.ts`, `useSessionNavigationAgentConnection.ts`, and `useSessionNavigationCommands.ts`. The tree controller is shared by the rendered selector and commands. The loopback bridge forwards a typed command to the mounted UI and returns its state after React applies the action; it has no second navigation model or direct execution path.

`navigation-commands.md` documents the connection descriptor and `scripts/agent-session-ui.mjs`. Available commands inspect, select/reveal, open a folder draft, expand/collapse, show more, move, pin/unpin, and return a deeplink. No provider messages are sent by these navigation commands.

After Computer Use was stopped by the user, nine commands passed through the running Windows application endpoint. Verification checked five-row disclosure, reveal, folder drafts, moves into a workflow with Added/Workflow ordering, pins, deeplinks, workspace/address preservation, and rejection of requests without the local token. Restart and running-instance URL-argument checks then passed through the same interface and confirmed persisted placement/pins. Actual pointer drag/drop and installed Windows URL-handler invocation remain outside the verified evidence.
