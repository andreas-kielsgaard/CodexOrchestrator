# OTP selection and Stop session validation

2026-09-08. Implemented in `C:/Users/user/.codex/worktrees/workflow-continuation-files` on
`codex/workflow-continuation-files`, HEAD `9502879`. Changes are uncommitted. The unrelated
untracked Codex defaults/steering plan was preserved.

## Implementation

- `components/otp/` owns the reusable selection dialog; `application/otp/` owns shared DTOs.
- Trigger-output, destination-action and profile-MCP adapters retain their selection rules.
  Workflow entry uses the same action adapter and catalogue as connection destinations.
- `otp_packages/workflow/stop_session.rs` selects one running destination session, newest
  by default or last addressed. No match produces a recorded no-op. It emits a typed stop
  request through `otp_api`, without a new MCP endpoint.
- `otp_host/session_control.rs` adapts existing Agent Session cancellation. The host validates
  node/session ownership and persists the exact invocation before cancellation. Attempts
  distinguish pending/requested/no-op/failed outcomes from observed terminal session state.
- Technical Settings contains Codex home profiles and OTP configuration tabs. The latter reads
  the existing imported registry and displays the real Workflow package, status and tools.

Continuation scope and Harness-derived file attribution are unchanged. Stop does not archive
or delete a session. The session remains available for another prompt.

## Automated checks

| Check | Result |
| --- | --- |
| Shared picker, settings, authoring, profiles, affected clients/navigation | 38 passed in 14 files |
| OTP packages | 6 passed |
| Workflow compiler/storage/file inputs | 14 passed |
| Agent Session integration | 18 passed; 2 live exercises excluded from the ordinary run |
| Harness engine | 33 passed |
| Session Events | 17 passed |
| TypeScript and Vite production build | Passed; existing large-chunk warning |
| Native binaries, test-fast profile | Passed; existing compiler warnings |
| Changed-file ESLint and `git diff --check` | Passed |

The five Rust groups total 88 passing tests. They used:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --features live-tests --lib <filter> -- --nocapture
```

Filters: `otp_packages::`, `workflows::`,
`agent_sessions::application::tests::repair_tests`, `harness_engine::`, `session_events::`.

```powershell
npm test -- src/components/otp src/features/technicalSettings src/features/workflowAuthoring src/features/executionConfiguration src/features/nativeProfiles src/app/App.sessionEventModel.test.tsx src/infrastructure/workflowAuthoring src/infrastructure/workflowInstances
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --profile test-fast --bins
```

Key coverage: distinct package/output identities; preview versus checked selection; cancel and
same-action preservation; unavailable choices; runtime/profile eligibility and external tools;
entry Stop configuration persistence; visible empty-prompt no-op; exact invocation targeting;
failure recorded after pinning; duplicate occurrence suppression; reopening stored outcomes;
and an already-finished invocation not canceling a newer turn.

Final logs are under `C:/Users/user/.codex/tmp/`:
`otp-picker-ui-tests-final.log`, `otp-picker-rust-regression-final.log`,
`otp-picker-build-final.log`, `otp-picker-native-build.log`, `otp-picker-lint-final.log`.

## Real Codex cancellation

The ignored `real_codex_stop_session_exercise` was explicitly run and passed in 25.48 seconds.
It uses real Codex processes with product Agent Session/Workflow services and a disposable
workspace/database. The proxy host is in-process; this is separate from the native UI inspection.

Observed a running PowerShell sleep command, dispatched the OTP Stop action, verified the
recorded invocation ID and terminal `Canceled` state, then prompted the same session and
observed `Completed`. A separate control session retained its single completed invocation.
The fixture shuts down its runtime and engine after the exercise.

Evidence: `C:/Users/user/.codex/tmp/otp-stop-live-20260908-02/`, including `stop-result.json`,
`sessions.json`, `attempts.json`, `history.json` and `live.sqlite`. Log: `otp-picker-live.log`.
The first run used a completion helper that incorrectly rejected the intentionally canceled
historical turn; the test was corrected to inspect the latest turn, then passed in a fresh root.

Reproduction requires a fresh `WORKFLOW_LIVE_EXERCISE_ROOT` and the compatible installed
CLI path in `WORKFLOW_LIVE_CODEX_PROGRAM`, then:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --features live-tests --lib real_codex_stop_session_exercise -- --ignored --nocapture
```

## Native walkthrough

Rebuilt and restarted **Codex Orchestrator [OTP Demo]**, using isolated app data and WebView
state under `C:/Users/user/.codex/tmp/otp-demo-20260908/`, with Vite on port 1490. Existing
demo recipe/profile/instance data was retained. Default Codex home displayed ready.

Screenshots and interaction in the native window verified:

- Settings tabs and actual imported `workflow` package, listing Workflow continuation,
  Handoff to agent, Invocation completed, Prompt agent and Stop session.
- Connection Set trigger modal, output-specific details, offered schema and preview without
  mutation; Escape retained the original trigger and returned focus to its button.
- Initial destination Set action modal using the same Prompt agent/Stop session catalogue.
- MCP multiple-selection modal in both Capability Profiles and Workflow node profiles,
  with the existing two MCP selections visible. Inspection was canceled without changing
  profile permissions or saving a new revision.
- Package collapse/keyboard expansion, reverse tab traversal and Escape/focus return.
- Picker layout at 1280 and 960 logical pixels of client width. Labels, details and footer
  controls remained visible; long details scroll within the modal.

The initial-destination disclosure originally overlapped the canvas toolbar because its
container assumed two grid rows. Flex sizing fixed the overlap and was rechecked visually.
The instance layout uses the same adjustment to accommodate action messages.

The app is left running on Technical Settings → OTP configuration. No native demo agent
was started or stopped during UI inspection; live cancellation used the disposable exercise.

## Limits

This is focused slice validation, not the full unrelated regression suite, a packaged release
or user acceptance. No formal screen-reader audit or below-minimum-width/mobile validation
was performed. Multiple-package and missing-package cases use component fixtures because
only the Base Workflow OTP is imported in the native product. Native MCP sidecar continuation
was not re-exercised live in this slice; its affected routing has integration coverage.
