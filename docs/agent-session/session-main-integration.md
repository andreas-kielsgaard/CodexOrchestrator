# Agent Session integration onto main

2026-09-09. Baseline: local `main` at `deecb2f7bb0c2678c8fbcef0e1184bf43e56d5f0`.

`codex/session-defaults-steering` was reconstructed on this baseline. The previous branch is retained as `codex/session-defaults-steering-with-otp-20260909` at `4e742314886b20985d9c66052fc65e7d8c45931d`. This is a curated Session port, not a merge of the OTP ancestry.

## Included behavior

- A saved default Capability Profile is required for new standalone Sessions. Profile settings supply defaults; direct user messages retain the native runtime's available choices.
- The attached Codex home supplies native defaults and inherited capabilities. Current defaults are resolved on continuation. Product MCP connections are additive.
- Codex app-server owns start, resume, steering, cancellation, and supported approval/input interactions. Steering is persisted on the active invocation and is idempotent by input identity.
- The Workflow engine receives a distinct accepted-steering notification. It creates no additional delivery or routing action.
- A Session without a directory receives an empty retained directory under `~/.codex-orchestrator/workspaces`. The sibling `skills` directory supplies product skills. Historical blank contexts require native recovery or explicit selection; they are not silently relocated.
- Harness bindings retain tool selections and resolve current MCP endpoints at launch, including after application restart.
- Session attention states, approval controls, diagnostics, and composer behavior retain the reviewed usability corrections.

## Integration boundaries

`agent_sessions/application` contains focused creation, configuration, invocation, interaction, addressing, and workspace use cases behind one application facade. The former separate profile application/client is removed; its consumers use the same Session application/client.

`runtime/codex/app_server` owns native protocol and configuration details. `runtime/processes` owns duplex process IO, supervision, and Windows process-tree lifetime. Neither owns Workflow routing.

Session repositories retain main's shared `ActiveDatabase` readers and managed writer. Address storage moved out of the Session Event adapter into the Session repository. Native profile binding and capability defaults use the same managed database boundaries. Schema 48 adds the workspace origin/default selection and migrates persisted MCP transport details to logical exposure policies atomically.

`active_app/sessions`, `execution_configuration`, and `session_notifications` compose services and observers. Managed MCP and terminal observers are registered before Session recovery.

Fresh native startup exposed an existing main composition omission: Sprint storage was initialized by test openers but not production database setup. Its existing initializer is extracted to `orchestration/sprint_runner_transition/storage.rs` and shared with `product_database/active_schema.rs`. Existing Bootstrap initialization is also composed there. This initializes main's existing functionality; it adds no workflow feature. A missing test-only Mutex import was repaired to compile the live-test configuration.

## Excluded work

OTP packages, host, actions, authoring UI, workflow continuation/file attribution, and unrelated Technical Settings changes are excluded. Main's workflow recipe/event contracts, Workflow graph and instance UI, repository catalog, Worktree Review, and existing orchestration behavior remain. Workflow UI changes are only Session client imports; compiler/authoring changes initialize the added defaults field in fixtures.

## Validation

- Frontend: 961 product tests covered. The initial run passed 960; the remaining editor fixture still referenced the excluded OTP picker. Restoring main's picker interaction passed all four tests in that file. Production TypeScript/Vite build passed.
- Rust: the complete initial sweep passed 698 of 706 tests. All eight failures were migration/default-profile fixtures or initialization mismatches corrected during integration. The final focused run passed 328 tests, with three opt-in helpers ignored, covering Session, native profiles, runtime, harness, execution configuration, workflows, and storage, including all eight corrected failures. The separate Session run passed 73 tests with two live helpers ignored.
- New regressions verify a main-v47 database upgrade/reopen preserves Session context and converts old MCP connections, and that accepted steering persists once without another Workflow delivery.
- Native inspector: 40 tests passed.
- Installed Codex CLI 0.144.0 executable contract passed against a local Responses fixture: steering, interruption, resume, and inherited defaults.
- Rebuilt Tauri app exercised through owned native command handles, without computer-use input: required default selection, initially empty workspace, approval and invalid-response recovery, two steering inputs and exact retries, refreshed model/reasoning/approval defaults, explicit sandbox mismatch/recovery, and restart persistence.
- A second native pass enabled main's Workflow MCP tool policy and completed a fifth invocation after restart on the original thread/workspace. Durable protocol events confirmed the managed MCP connection was ready after restart. The evidence collector initially expected a provider `tools` field absent in this CLI's request format; the result was verified read-only from the persisted invocation and MCP startup events, and the collector was corrected.
- Wide and narrow native WebView captures were inspected for the default-profile and approval flows. This is not a keyboard/accessibility acceptance claim.

The live passes used disposable Codex homes and a local provider fixture, not real-model quality evaluation. CLI version was verified from its native Session metadata. Local logs, commands, captures, and result records are retained under `.dev/session-main-port-20260909`. No remote push or merge into main was performed.
