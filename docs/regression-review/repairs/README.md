# Regression repairs

7 September 2026. Repair commit `c1b89b9` was fast-forwarded into `codex/harness-ux-workflow-convergence`; its existing worktree now uses that branch. The original review reports and screenshots remain the pre-repair baseline.

## What changed

| Finding                                  | Repair                                                                                                                                                                                                                                                                          | Main files                                                                                                                                              |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1: second message blocked               | Standalone creation pins a profile before the first launch. Later messages use that same profile. A first-message override does not become a default. Old unprofiled Sessions remain readable, with sending disabled.                                                           | `agent_sessions/application/session_profile.rs`, `transport/profile.rs`, `AgentSessionScreen.tsx`                                                       |
| B2: handoffs not wired                   | Normal completion and the managed MCP handoff now reach the new event service. Each connection has a stored attempt. A repeated completion does not launch twice. A failed handoff does not change the sender's completed result.                                               | `workflows/event_sources.rs`, `workflows/mcp.rs`, `active_app.rs`                                                                                       |
| B3, F5, U2: instances and worktrees lost | Create/list/load use stored instances. Each keeps its name, recipe snapshot and chosen worktree. Creating one launches nothing. Node requests address that stored instance; new Sessions receive its working directory.                                                         | `workflows/instances.rs`, `execution_transport.rs`, `application/workflowInstances.ts`, `WorkflowInstancePanel.tsx`, `RecipeInstanceCreationDialog.tsx` |
| B4: shared edits block pinned Sessions   | Runtime compilation carries a creation intent. The Capability Profile is read only if a new Session is needed. Existing Sessions keep their pinned settings after profile edits or deletion.                                                                                    | `execution_configuration/creation_intent.rs`, `workflows/authoring.rs`, `compiler.rs`, `agent_sessions/session_event_adapter.rs`                        |
| B5: impossible source/trigger pairs      | Activation rejects unsupported pairs and group-completion triggers. Editors offer sources for the selected trigger and show invalid loaded choices. File references read contents within the chosen worktree. Fixed prompts retain recipe-field references in delivery records. | `workflows/authoring.rs`, `prompt_content.rs`, `WorkflowConnectionEditor.tsx`                                                                           |
| F1, F2, F6: drafts lost                  | App-owned draft workspaces survive screen changes. Late save replies retain newer typing and do not change selection. Activation uses the saved revision without replacing the local draft. Close/reload warns about unsaved edits.                                             | `components/draftWorkspace.ts`, `App.tsx`, both authoring screens, `tauriDraftCloseGuard.ts`                                                            |
| F3, F4: invalid node choices             | Node controls use the selected Capability Profile as their ceiling. Removed defaults stay visible as unavailable. Runtime-locked values are disabled and inherited.                                                                                                             | `WorkflowNodeEditor.tsx`, `CatalogSelect.tsx`, execution-configuration presentation                                                                     |
| F7, F8: Session details missing or stale | Session identity view/edit is restored. Delivery records refresh after storage changes and through Refresh; read errors are shown and old replies ignored. Instance notifications also cover failures before a delivery exists.                                                 | `AgentSessionExecutionSettings.tsx`, `ProfiledSessionPane.tsx`, `useSessionDeliveries.ts`, event and instance clients                                   |
| U1: canvas removed                       | A controlled canvas restores placement, drag, connection selection, node copying, deletion, start-node choice and undo/redo. The node and connection editors open beside it.                                                                                                    | `WorkflowCanvas.tsx`, `WorkflowAuthoringScreen.tsx`                                                                                                     |
| U3–U5: layout                            | Small screens retain recipe and instance controls. Collapsed content is hidden despite feature grid rules. Checkbox sizing no longer inherits text-input sizing. Instance styles do not alter the embedded conversation form.                                                   | `workflowAuthoring.css`, `workflowCanvas.css`, `collapsibleSection.css`, `catalogSelect.css`, `styles.css`                                              |

Rust paths above are under `src-tauri/src/`; frontend feature paths are under `src/features/`.

## Ownership and small changes to the plan

- `SessionProfileResolver` remains the shared pure resolver. The Session application owns standalone birth; the addressed adapter retains atomic Session/address storage. No new low-level creation framework was needed.
- Workflow instances and attempts share one small SQLite store. Session Events still own addresses, prompt materialization and delivery outcomes. Instance membership is read from the existing address directory, not a second membership table.
- The creation intent is `session_creation_intent/v1`. The existing resolved request remains useful for preview and technical tests; mounted instance execution uses the intent.
- The technical MCP engine accepts a pinned Session Profile directly. It reuses its existing binding table and proxy without creating a Role or mixed Harness catalogue entry. Old provenance columns are unused for these bindings. The receiver derives Workflow addressing from the trusted Session.
- Post-store notices are separate: `session-event-recorded` for delivery readers and `workflow-instance-updated` for instance readers, including preparation failures.
- The canvas owns gestures and layout only. The screen owns recipe edits; clients own transport. `DraftWorkspace` owns in-app draft lifetime and bounded undo history, not server persistence.
- The native close guard is an injected window port. Its one added permission allows the main window to finish closing after that check. It adds no provider or filesystem authority.

These file splits are practical choices for this repair, not a requirement to retain every file as the product evolves.

## Checks

- Nine new Rust integration tests use real SQLite, the real Session application, real event adapter/compiler and a recording runtime. They cover first/second sends, per-message choices, pinned settings after deletion, normal completion, application-event delivery, instance isolation/reopen, recipe pinning, real file reads, invalid sources and the managed MCP route.
- The MCP test uses the real binding engine and HTTP proxy in-process. Only the child-process sidecar transport and inference runtime are replaced. It checks tool exclusion and rejects injected routing before receiver launch.
- New frontend tests cover draft races, navigation, canvas edits, node ceilings, identity edits, delivery refresh/errors, stale instance replies, first/next messages, window-close decisions and instance command payloads.
- The full Rust run reported **711 passed, one timeout, two ignored**. The timeout was `work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness`, in the existing orchestration tests. Its isolated rerun passed. This is not an all-green full-suite result.
- TypeScript/Vite build and native `cargo build` passed. Vite still reports its large-chunk warning; Rust reports unused legacy/library items. Neither was suppressed or turned into a cleanup project.
- The [browser checks](run-browser.mjs) passed at **1280, 958, 850 and 640 pixels**, with no page errors. They use real components and fake clients. See [recorded results](evidence/results.json).
- The browser evidence was refreshed on 8 September after the Workflow instance screen moved to the shared node canvas. The reload check now reopens both the stored instance and its fixture Session.

The final frontend test count and final rerun results are recorded in [verification](verification.md). Do not add focused-test counts to full-suite counts.

The later [pre-merge audit](pre-merge-audit-2026-09-08.md) records the current node-based instance flow, fresh browser evidence, and the remaining merge choices.

## Replay and screenshots

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib repair_tests
cargo build --manifest-path src-tauri/Cargo.toml
node docs/regression-review/repairs/run-browser.mjs
```

The browser runner needs Playwright and Edge. If Playwright is outside the repo, set `REVIEW_NODE_MODULES` to its `node_modules` directory. `REVIEW_BROWSER_CHANNEL` can choose another installed Playwright browser channel. It defaults to port 2382; set `REVIEW_PORT` when another isolated port is needed. The runner uses an isolated browser context and closes its processes afterward.

- [Flow canvas](evidence/01-flow.png)
- [Connection editor](evidence/02-connection.png)
- [Collapsed node sections at 958 px](evidence/03-node-958.png)
- [Small-screen controls at 640 px](evidence/03-node-640.png)
- [Instance creation at 958 px](evidence/04-create-958.png)
- [Stored instance](evidence/05-saved-instance.png)
- [Session within an instance](evidence/06-instance-session.png)

## Still outside this proof

No live provider was called. No old data was migrated. No provider setup, new CLI controls, custom scripting/compaction, generic retry system, run graph, Epic redesign, broad legacy deletion or merge with adjacent branches was added.

The native executable was built, but a fresh native-window create/reopen walkthrough and user review have not been performed. The old isolated runtime manifest is stale and has not been used to claim this repair is mounted. Browser storage is a fixture; actual persistence is proved by the Rust SQLite tests. Native window-close behavior has a port test, not a native-window observation.
