# Tauri operation catalogue

## Boundary shape

The desktop product is one Rust crate. `active_app.rs` constructs backend services and registers Tauri operations; TypeScript infrastructure adapters call them through `@tauri-apps/api/core::invoke`. Most state is returned through snapshot queries. Two push-event families were found: Agent Session updates and Epic initiation confirmation.

The operational inspection line registers 45 release operations and 21 additional debug-only operations.

## Root operations

| Operation | Purpose | Frontend/reachability |
| --- | --- | --- |
| `app_metadata` | Report app name, storage mode, and runtime label | Functional root command |
| `load_open_task_dashboard` | Load earlier Task dashboard | Registered but always fails closed |
| `register_task_worktree` | Register earlier task worktree anchor | Registered but always fails closed |
| `register_task_repo` | Register earlier task repository | Registered but always fails closed |
| `discover_task_repos` | Discover Git repositories for earlier Tasks | Registered but always fails closed |
| `create_open_task` | Create earlier Task | Registered but always fails closed |
| `update_open_task` | Update earlier Task | Registered but always fails closed |
| `archive_open_task` | Archive earlier Task | Registered but always fails closed |
| `load_task_run_detail` | Load earlier Task run detail | Registered but always fails closed |
| `start_codex_task_run` | Start earlier task-based Codex run | Registered but always fails closed |

The nine legacy operations enter `ensure_legacy_tasks_available`, which returns an error unconditionally. Their TypeScript wrappers remain in `src/infrastructure/tauriCommands.ts` but are not part of product composition.

## Agent Session operations

| Operation | Product responsibility | TypeScript adapter |
| --- | --- | --- |
| `create_agent_session` | Create a durable empty Session | `tauriAgentSessionClient` |
| `list_agent_sessions` | List and filter Session summaries | `tauriAgentSessionClient` |
| `load_agent_session` | Load authoritative Session history | `tauriAgentSessionClient` |
| `send_agent_session_message` | Persist and launch/resume an ordinary user invocation | `tauriAgentSessionClient` |
| `cancel_agent_invocation` | Request runtime cancellation | `tauriAgentSessionClient` |

The same transport publishes `agent-session-update`. The frontend subscribes before sending because a fast child can persist and emit before acknowledgement returns.

## Native-profile operations

| Operation family | Operations | Notes |
| --- | --- | --- |
| Query and registration | `load_native_profile_query`, `register_native_profile`, `create_dedicated_native_profile` | Product Technical Settings |
| Selection and mode | `select_native_profile`, `select_native_profile_execution_mode` | Durable profile and execution-mode choice |
| Danger authority | `authorize_native_profile_danger_full_access`, `revoke_native_profile_danger_full_access` | Exact profile filesystem identity is significant |
| Authentication | `request_native_profile_login`, `refresh_native_profile_readiness` | Request and observation remain separate |
| Sandbox | `request_native_profile_sandbox_initialization`, `confirm_native_profile_sandbox_initialization`, `verify_native_profile_preprovisioned_sandbox`, `confirm_native_profile_preprovisioned_sandbox_adoption` | Separates request, observed confirmation, verification, and adoption |
| Canaries | `run_native_profile_workspace_write_canary`, `run_native_profile_danger_full_access_canary` | Separate readiness evidence for the two execution modes |
| MCP reporting | `probe_native_profile_mcp_reporting`, `reconcile_native_profile_mcp_reporting` | Initial frontend client calls reconcile from its visible Start probe action; no TypeScript caller for the probe command was found in the baseline |

All 17 operations are implemented in `native_profiles.rs`, together with their service, schema, CLI runner, policy, MCP server, and DTOs.

The backend probe lifecycle was hardened after the initial anchor and is present at current tip `9240364`. The frontend action still reaches reconciliation rather than request creation, so the end-to-end classification remains incomplete.

## Orchestration operations

| Operation | Responsibility | Product path |
| --- | --- | --- |
| `send_managed_plan_builder_message` | Send managed user discussion with Plan Builder Harness and MCP | Plan Builder-specific Agent Session client |
| `request_managed_plan_builder_action` | Send the application-owned Build request | Plan Builder Build action |
| `reconcile_managed_plan_builder_session` | Bind/recover durable draft and Session association | Plan Builder draft lifecycle |
| `load_managed_plan_builder_harness_inspection` | Load effective Plan Builder Harness and launch evidence | Contextual Harness inspection |
| `update_epic_planning_draft_title` | Update active draft title | Plan Builder lifecycle |
| `cancel_epic_planning_draft` | Terminally cancel a draft | Plan Builder lifecycle |
| `request_epic_initiation_confirmation` | Create a button-originated confirmation request | Shared confirmation UI |
| `resolve_epic_initiation_confirmation` | Record explicit user decision and apply when confirmed | Shared confirmation UI |
| `load_orchestration_native_query` | Load durable orchestration snapshot | Primary product read path |
| `load_scoped_file_review` | Resolve an opaque stored review reference | File Review data source |
| `request_contextual_file_review` | Produce/request review evidence for a Sprint | Frontend caller exists; release state is composed as unavailable in the initial baseline |
| `load_epic_bootstrap_transition_query` | Load bootstrap/Epic Runner transition state | Product read composition |
| `load_sprint_runner_transition_query` | Load Sprint/execution transition state | Product read composition |

The confirmation coordinator also publishes `orchestration://epic-initiation-confirmation` for the modal and waiting agent tool.

## Debug Worktree Review operations

The 21 debug-only operations form several related surfaces:

- Catalogue: `list_human_review_worktrees`, `list_human_review_instances`.
- Preparation and operation progress: `prepare_human_review_instance`, `human_review_operation_progress`, `list_human_review_operation_progress`.
- Detail and comparison: `human_review_instance_detail`, `human_review_instance_comparison`.
- Proof navigation/presentation: `human_review_launcher_proof_navigation`, `human_review_launcher_detail_navigation`, `human_review_launcher_proof_presentation`, `worktree_review_proof_navigation`.
- Worktree Build: `mark_worktree_build_ready`, `worktree_build_context`, `worktree_build_detail`, `worktree_build_comparison`.
- Lifecycle control: `build_human_review_instance`, `start_human_review_instance`, `status_human_review_instance`, `focus_human_review_instance`, `stop_human_review_instance`, `recover_human_review_instance`.

These commands are used by `tauriHumanReviewLauncher.ts`, `tauriWorktreeBuild.ts`, and the debug review composition. They are not available in release builds.

## Product Decisions sibling-line operations

The Product Decisions/navigation line adds seven release Tauri operations:

| Operation | Responsibility |
| --- | --- |
| `accept_product_decision_version` | Accept an exact durable version |
| `load_product_decision_current_query` | Load current Product Decisions for an Epic |
| `load_product_decision_history` | Load version history for one decision |
| `start_product_decision_correction_conversation` | Create the managed correction conversation |
| `send_product_decision_correction_message` | Continue correction discussion |
| `save_product_decision_correction_proposal` | Persist an exact proposed correction |
| `accept_product_decision_correction_proposal` | Accept the validated proposed passage as a new version |

These operations are absent from the current operational inspection line because Product Decisions and Native Profiles remain on divergent histories.

## Architectural observations

- Tauri transport is relatively narrow for Agent Sessions and orchestration, but `native_profiles.rs` and legacy `lib.rs` combine transport with implementation.
- Registered does not mean usable: legacy commands fail closed, contextual review production is unavailable in release composition, and debug operations do not exist in release builds.
- A caller does not prove a complete capability: the native MCP action currently targets reconciliation while probe creation is exposed separately.
- Snapshot operations dominate. Agent update and initiation confirmation are the notable event-driven paths.
