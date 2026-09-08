# Repository-based Agent Sessions

Status: implementation plan only, 2026-09-08.

Baseline inspected: local `main`, `f5542a4`. Existing launcher edits and presentation artifacts are unrelated and remain untouched. This plan replaces the Agent Sessions Epic/Sprint integration; it does not require preserving that integration.

## Requested behavior

- Show every registered local repository as a folder, with collapsible **Workflows** and **Sessions** sections. Workflows contains instance folders, not workflow definitions.
- Ordinary sessions can be placed in a repository, a workflow-instance folder, or **Unfiled**. Placement never changes their working directory, runtime context, harness, or workflow ownership.
- A repository folder's new-session action opens an ordinary draft whose first send uses that repository's main working tree. The action on a workflow-instance folder does the same, placing the session in that instance's manually added area.
- Show manually placed sessions above the instance's own sessions, separated by a label/divider and ownership markers. A session owned by another workflow retains its original ownership marker.
- Folder creation actions appear on hover and keyboard focus. Repository-level creation targets its Sessions section; instance-level creation targets that instance. The global action creates an unfiled session.
- Support drag-and-drop between these locations, pin/unpin, and a session context menu with **Pin/Unpin**, **Move to**, and **Copy deeplink**. Pinned entries are shortcuts to the same sessions, not relocated sessions.

### Five sessions and Show more

“Messages” means session rows in the sidebar, not turns in the transcript.

- Initially show five session rows per session-containing folder, and five in Unfiled. Pinned shows all.
- In an instance folder, concatenate manually placed sessions followed by its owned sessions, then take five. This is one limit for the whole folder, not five per group. Labels and dividers do not count.
- Order sessions by most recently updated first within each group, with session ID as the tie-breaker. Order pinned shortcuts by pin time. Moving or pinning must not update conversation activity timestamps.
- **Show more** reveals the remaining sessions in that folder. It does not fetch another page or expand other folders. Repository headers and lists of workflow-instance folders are not subject to the session-row limit.
- Keep disclosure state in UI memory. Selecting a session through a deeplink reveals its containing folder and, when necessary, its remaining rows. An explicitly collapsed folder can stay collapsed afterward.
- Treat Show more as a keyboard-operable action in the visible navigation sequence. Use one derived visible-row list for rendering and keyboard movement.

## Registration dependency: use the existing work

The referenced task, **Worktree Review: Worktree Cleanup** (`01a0395c-f32c-78c0-8da1-521197812474`), implemented registration in the working tree at:

`C:/Users/user/.codex/worktrees/worktree-review-durable-architecture/Codex Orchestrator`

The inspected branch is `codex/worktree-review-durable-architecture`, committed HEAD `dc89528`. That HEAD is not an ancestor of current main. Registration is present in tracked modifications and new files, not in a registration commit. The task was active during inspection. Its last completed report describes tests and implementation; those checks were not rerun for this plan.

Relevant existing work there:

- `src-tauri/src/repository_context/identity.rs`: canonical Git common-directory identity.
- `src-tauri/src/repository_context/worktrees.rs`: local worktree enumeration.
- `src-tauri/src/worktree_review/domain/source.rs`: `ReviewRepository`, including `anchor_root` and `common_directory`.
- `src-tauri/src/worktree_review/storage/repositories.rs`: registered repository persistence and listing.
- `src-tauri/src/worktree_review/state.rs`: register/select/list operations.
- `src-tauri/src/worktree_review/repository_registration.rs`: manual/Codex registration and discovery coordination.
- `src/features/worktreeReview/RepositoryRegistrationModal.tsx`: the existing registration UI.

Use that catalog as the production dependency. This supersedes the earlier suggestion to build a new reader around legacy `repos` as the long-term source. Registration should be integrated into the implementation baseline before production wiring of this view; integrating its active dirty branch is separate work, not performed by this plan. Frontend projection and organization logic can be developed against its small contract meanwhile.

Expose a neutral `RegisteredRepositoryCatalog` with a list operation returning repository ID, name, canonical common directory, and an explicit main working-tree path. Its adapter reads the existing registered catalog. It must not invoke the registration overview, Codex discovery, GitHub discovery, or change Worktree Review's selected repository when listing folders.

The branch's `anchor_root` is documented as a usable checkout. Registration normally chooses the first enumerated worktree but can fall back to the candidate checkout. Make main-working-tree resolution an explicit shared operation using `RepositoryContext`; do not silently equate any anchor with the main working tree. Test registration from a linked worktree and creation at the main tree.

Current main stores workflows against legacy repository IDs but also records `repository_git_common_directory`. Associate existing workflow folders with the new catalog through that recorded common-directory identity using the shared path identity rules. Do not match names or arbitrary working-directory prefixes. Adapt the existing workflow target source to use the same catalog so newly created instances carry its canonical repository ID. Keep the current `ResolvedRepoBranchWorktreeTarget` contract and temporary selector UI.

There is no need to merge the repository catalogs into one database in this feature. The new catalog uses `worktree-review.sqlite`; sessions/workflows use the active database. Compose their read results through the catalog interface. All session-organization writes stay in the active database.

## Data and application boundary

Add one active-database table, `agent_session_organization`:

| Field | Purpose |
| --- | --- |
| `session_id` | Primary key and foreign key to the session |
| `placement_kind` | `default`, `unfiled`, `repository`, or `workflow_instance` |
| `placement_target_id` | Repository/instance ID for the corresponding target kind |
| `pinned_at` | Nullable pin timestamp |

Use a check constraint for the target-kind/target-ID pairing. No row is equivalent to default placement and unpinned. An explicit unfiled value allows a workflow-owned session to be visually moved out without losing ownership. Pinning a default-placed session preserves default placement.

Resolve display location once: explicit placement wins; otherwise use recorded workflow ownership; otherwise Unfiled. Existing ordinary sessions remain unfiled until moved. Repository/instance names and paths are read from their owners rather than duplicated in organization rows.

Keep actual ownership in `workflow_instance_sessions`. Organization commands never write that table or bind a workflow harness. The workflow's session counts and delivery logic continue to read actual ownership.

Expose a small organization client:

1. `loadNavigation()` returns registered repos, lightweight instance facts, session summaries/ownership, and organization records.
2. `sendFirstMessage(target, message)` creates an ordinary session with its initial placement, then sends through the existing ordinary session application.
3. `moveSession(sessionId, target)` changes placement only.
4. `setPinned(sessionId, pinned)` changes pin state only.

For first send, resolve the target repository and main path before entering the managed write. Prepare the ordinary session using shared creation logic, insert session and organization together, commit, then invoke the existing send operation with that session ID. No provider or filesystem operation belongs inside the transaction.

Use a small folder-bound AgentSessionClient adapter, following the existing workflow-client adaptation pattern. It redirects only a draft's first send to the organization boundary. Existing-session sends delegate to the ordinary client. Folder placement does not enter the transcript or runtime APIs.

Load navigation facts in a few bounded queries. Do not load transcripts, recipes, workflow execution plans, or registration-discovery results to render the sidebar. Apply the five-row limit in the pure frontend projection; backend pagination is unnecessary for this scope.

## Change, delete, create

Paths below are relative to this repository. New file names define responsibilities; colocate their focused tests.

### Frontend

| Action | Files | Result |
| --- | --- | --- |
| Replace | `src/application/agentSessionNavigation.ts` | Pure repository/instance/placement projection. Delete Epic reference traversal, semantic-role placement, planning-draft inputs, and the old alphabetical tree sorter. |
| Create | `src/application/agentSessions/organization.ts` | Target union, navigation facts, and organization-client contract. |
| Create | `src/application/agentSessions/folderAgentSessionClient.ts` | First-send adapter; no workflow ownership behavior. |
| Create | `src/infrastructure/agentSessions/tauriSessionOrganizationClient.ts` | Thin transport mapping for the four organization operations. |
| Replace/split | `src/features/agentSessions/useAgentSessionController.ts` | Move conversation loading, sending, canceling, subscriptions, and transcript state into `useAgentSession.ts`. Remove the combined controller and its `skipCollection` mode after updating callers. |
| Create | `src/features/agentSessions/useSessionNavigation.ts` | Own navigation loading, selection/new-draft target, move/pin actions, and refresh after successful writes. Use explicit draft state rather than treating null selection as permission to select the first existing session. |
| Simplify | `src/features/agentSessions/AgentSessionScreen.tsx` | Compose the navigation and existing workspace. Remove Epic/Sprint folder inputs, the Related product views chooser, and the Epic-specific return banner. |
| Split | `src/features/agentSessions/SessionSelector.tsx` | Keep the sidebar shell. Extract `SessionTree.tsx` for rows and inline folder actions, `useSessionTree.ts` for disclosure/visible rows/keyboard behavior, and `SessionContextMenu.tsx` for the three menu actions. Avoid nested buttons. |
| Split | `src/features/agentSessions/agentSession.css` | Move sidebar/tree/menu styles and their responsive rules to `sessionNavigation.css`; retain conversation styles in the existing file. |
| Change | `src/app/App.tsx`, `src/bootstrap/productApplicationComposition.ts` | Inject the organization client and route selected-session navigation. Do not place catalog joins, folder rules, or mutations in App. |
| Relocate surviving types | `src/application/orchestrations/navigation.ts` | Move old `AgentSessionProductLocation`/`AgentSessionProductOrigin` types here only because other existing orchestration screens still import them. Update imports directly; keep no compatibility exports in the new sidebar model. |
| Create | `src/application/agentSessions/deepLinks.ts`, `src/infrastructure/agentSessions/tauriSessionDeepLinks.ts` | Small URL formatter/parser and incoming-link adapter. Wire through existing typed app navigation. |
| Update | Relevant barrel files, recorded clients, fixtures, navigation tests, and existing documentation | Remove stale APIs and assertions; add the requested repo/instance behavior. Mark superseded navigation documentation as historical without rewriting past evidence. |

Keep `AgentSessionWorkspace`, the transcript projector, composer, and harness controls as the conversation surface. The pure workspace hook remains reusable by existing embedded sessions. Extract a neutral controller type alongside that hook instead of deriving workspace types through `Pick`/`Omit` of a combined sidebar controller.

Use distinct row IDs for pinned shortcuts and folder entries, both referencing the same session ID. Selection and pin state belong to the session; keyboard focus belongs to the row.

### Backend and catalog integration

| Action | Files | Result |
| --- | --- | --- |
| Create | `src-tauri/src/repository_catalog.rs` | Small shared catalog contract and registered-repository projection; reuse existing canonical identity types. |
| Create adapter | `src-tauri/src/worktree_review/repository_catalog.rs` after registration integration | Read the existing registered inventory and expose main-working-tree resolution. No second catalog or discovery implementation. |
| Change | `src-tauri/src/worktree_targets_temp/mod.rs` | Consume the shared catalog and RepositoryContext for workflow target choices; remove its duplicated legacy repository-row reader. Retain the target contract. |
| Extract | `src-tauri/src/agent_sessions/application/creation.rs` from `application/lifecycle.rs` | Shared session construction, normalization, and first-message title derivation. Ordinary and organized creation use one implementation. Keep execution, cancellation, runtime repair, and launch authority in their current runtime path. |
| Create | `src-tauri/src/agent_sessions/organization/mod.rs` | Organization types, catalog/ownership composition, and the four use cases. |
| Create | `src-tauri/src/agent_sessions/organization/repository.rs` | Navigation read queries, organization schema, move/pin writes, and atomic session-plus-placement insertion through ActiveDatabase. Reuse the existing session insert helper. |
| Create | `src-tauri/src/agent_sessions/organization/transport.rs` | Thin native commands and DTO conversion. |
| Change narrowly | `src-tauri/src/agent_sessions/repository/mapping.rs` and module exports | Share the existing validated session insertion without opening nested managed writes or copying its SQL. |
| Change | `src-tauri/src/storage.rs` | Register the additive organization migration for current and fresh databases. |
| Change | `src-tauri/src/active_app.rs` | Construct and register the catalog adapter and organization service. Keep behavior out of composition. |
| Create/change | `src-tauri/src/session_deep_links.rs`, Tauri config/capabilities, Cargo/npm manifests | Native URL reception and application scheme registration. |

No new behavior goes into quarantined `src-tauri/src/lib.rs`; only necessary module declarations are appropriate there.

## Monoliths worth addressing in this feature

- **`useAgentSessionController.ts`: split now.** It duplicates collection state and has a mode flag to suppress that same responsibility for embedded sessions. Removing that split personality directly clarifies folder drafts and refresh behavior.
- **`SessionSelector.tsx`: split now.** Rendering, tree traversal, selection reveal, focus management, and keyboard rules currently live together. The three new interactions should not accumulate in one component.
- **`agentSessionNavigation.ts`: replace its old model.** It mixes product navigation contracts with tree construction. Move still-used orchestration types to their owner and keep the new projector free of Epic/Sprint concepts.
- **`application/lifecycle.rs`: extract creation only.** Organized first send needs the same creation rules. This does not justify rewriting the complete execution lifecycle.
- **Registration branch `worktree_review/state.rs`: extract repository operations as part of catalog integration.** Move register/select/list methods together into a focused `registered_repositories.rs` implementation, leaving state/bootstrap access in state.rs. Existing discovery coordination stays in repository_registration.rs. Share existing storage through the catalog adapter; relocating the whole Worktree Review database is unnecessary.
- **`App.tsx`, `WorkflowScreen.tsx`, and the workflow application/repository files: keep new behavior out.** They are large, but this sidebar does not require a broad rewrite of workflow editing or execution. Query instance IDs, names, repository identity, and actual session associations through the focused organization reader instead of calling full-instance projection.

Delete superseded implementations and their replacement-only tests in the same change. Do not retain old and new sidebar modes, compatibility switches, or duplicate collections. Existing tests for transcript behavior and unaffected embedded consumers continue to provide coverage.

## Deeplinks and native dragging

Use a local application URL such as `codex-orchestrator://sessions/<session-id>`. It identifies the application session, never the provider's external thread ID. Copy through the existing clipboard abstraction. Opening it routes to Agent Sessions and reveals the selected row without creating a session or sending a message.

Use Tauri's [deep-link integration](https://v2.tauri.app/plugin/deep-linking/) for cold and already-running application entry. Keep reviewed-build application identities separate using the existing runtime setup; do not invent a cross-instance routing system.

For HTML5 dragging on Windows, configure the relevant webview's `dragDropEnabled: false`, as required by [Tauri](https://v2.tauri.app/reference/config/#dragdropenabled). Move to in the context menu uses the same mutation as dragging. Test the actual packaged/native window, not only browser events.

## Implementation order and acceptance

1. Establish a baseline containing the registration implementation and its existing prerequisites. Add the shared catalog adapter, explicit main-tree resolution, and workflow-target catalog alignment. Verify a registered repo and an existing workflow project to the same folder.
2. Implement organization persistence, lightweight navigation query, shared session creation, and first-send adapter. Verify ordinary creation at the repo main tree, persistence after reopen, and unchanged runtime/workflow facts after move/pin.
3. Replace the old navigation projection and split the frontend controllers/tree. Deliver folders, ownership distinction, hover creation, five-row disclosure, dragging, and the context menu together. Update recorded fixtures so the complete flow is reviewable without provider traffic.
4. Wire native deeplinks and test the visible flow. Remove superseded code, exports, tests, and documentation claims. Run the normal frontend build and focused Rust/frontend checks for the touched boundaries.

Acceptance cases:

- Two registered repositories, including an empty one; workflow instances under the correct repository; ordinary sessions and Unfiled remain available.
- Six ordinary sessions show five and Show more; revealing one folder leaves the others at five. Six pinned sessions all appear.
- Mixed manual/owned sessions use one five-row limit, manual first, with a visible distinction. Opening a hidden session by deeplink reveals it.
- Create from repo and instance folders, including when the repository was registered from a linked checkout; first send uses the main tree and creates no workflow association.
- Move a session between repos, into/out of an instance folder, and to Unfiled; pin/unpin; reopen storage. Confirm unchanged working directory, runtime binding, harness, and actual workflow membership.
- Folder actions and Show more work by keyboard; right-click Copy deeplink opens the same session in both cold and running native application flows.
- Existing conversation rendering, sending/canceling, and embedded-session callers still work after the controller split.

The scope is the requested local happy flow. It adds no custom folders, bulk operations, arbitrary row ordering, folder pinning, search, server pagination, repository discovery UI, cross-machine links, generalized migration framework, or workflow lifecycle changes. Use ordinary errors and existing persistence safeguards; add no recovery subsystem for this feature.
