# OTP usability audit — 2026-09-08

The exercised OTP flows work. Two UX discrepancies remain: Stop uses prompt-oriented empty-state
guidance, and connection action selection comes after the prompt fields it controls. This audit
adds review tooling; it does not change either product behavior.

## Instance and method

- Worktree: `C:/Users/user/.codex/worktrees/workflow-continuation-files`, branch
  `codex/workflow-continuation-files`, HEAD `9502879` plus the existing dirty OTP implementation.
- Isolated review PID: `45924`; executable: `src-tauri/target/test-fast/codex-orchestrator.exe`.
- WebView endpoint: `http://127.0.0.1:9231`; page: `http://127.0.0.1:1490/`.
- Review data: `C:/Users/user/.codex/tmp/otp-usability-20260908-01/app-data`. Both databases were
  copied using SQLite's backup API with read-only source connections. The original demo PID
  `38560` and its data remain available. The other task's application was not targeted.
- Viewport: 1280 × 820 CSS pixels, DPR 1.25. Screenshots are actual WebView compositor output.
- After the request to use CLI handles, interactions and captures used the exact-owner WebView
  companion. No desktop focus or OS input was needed. Product edits were made through its UI;
  database verification was read-only. No agent inference was started.

Evidence root: `C:/Users/user/.codex/tmp/otp-usability-20260908-01`. Each numbered capture has
a PNG and JSON rendered-state receipt; interactions also have an `-input.json` receipt.

## Walkthrough

1. **Trigger selection: passed.** The modal groups three outputs under Workflow. Previewing
   Handoff warns that applying it removes the unavailable `outputFiles` input. Cancel leaves
   both `sourceNode` and `outputFiles` selected and restores focus to Set trigger.

   ![Trigger replacement warning](C:/Users/user/.codex/tmp/otp-usability-20260908-01/06-handoff-preview.png)

2. **Destination selection: passed, with discovery friction.** Both a connection and User
   request destination offer Prompt agent and Stop session through the same picker. Stop hides
   prompt editing and exposes ordering of running sessions. Both selections persisted in draft
   v2 after reload and activation. The connection action button initially sits at y=1868 in
   an 820-pixel viewport, below the prompt editor.

   ![Connection before action selection](C:/Users/user/.codex/tmp/otp-usability-20260908-01/04-connection.png)

   ![Stop selected for the connection](C:/Users/user/.codex/tmp/otp-usability-20260908-01/10-stop-applied.png)

3. **Stop with no running session: passed, with misleading guidance.** Created a review
   instance pinned to v2 and ran its Stop action with no prompt. The app reports “No running
   destination session; nothing to stop.” Workflow actions and SQLite retain the attempt,
   with no error, session request or event delivery. However, the same screen tells the user
   to send a request to start a session, which the selected action cannot do.

   ![Stop result and conflicting empty-state guidance](C:/Users/user/.codex/tmp/otp-usability-20260908-01/23-stop-noop.png)

4. **MCP capability and node pickers: passed.** Both group the two available MCP tools under
   Workflow and show argument details. Unchecking Workflow continuation in the capability
   profile changes the selection count; Cancel restores the original selection. On the node,
   unchecking Handoff and applying changes its summary to Workflow continuation. Saving creates
   draft v3; read-only SQLite verification confirms the node restriction, while active v2 and
   the already-created instance retain their earlier configuration.

   ![MCP picker](C:/Users/user/.codex/tmp/otp-usability-20260908-01/26-profile-mcp.png)

5. **Technical Settings: passed.** Codex home profiles and OTP configuration are separate tabs.
   OTP configuration displays Workflow as Imported and lists Workflow continuation, Handoff to
   agent, Invocation completed, Prompt agent and Stop session. Refresh preserves this result.

   ![Discovered Workflow package](C:/Users/user/.codex/tmp/otp-usability-20260908-01/39-final-state.png)

## Findings and proposed fixes

| Priority | Finding | Change location | Proposed fix |
| --- | --- | --- | --- |
| P2 | Stop has “Send a request to a node to start” empty-state guidance and a “Send to node” heading. | `src/features/workflowAuthoring/WorkflowInstancePanel.tsx:115` and `:176` | Use action-aware guidance and label the operation with the configured action. Explain that Stop requires an existing running session. |
| P3 | Action selection sits below prompt editing; choosing Stop requires traversing fields that disappear. | `src/features/workflowAuthoring/WorkflowConnectionEditor.tsx:100` and `:123` | Place Destination action before its conditional Prompt logic section. |

No functional blocker appeared in these exercised paths. Raw JSON detail is verbose but the
picker's selection and footer remain readable at the tested size; this was not treated as a bug.

## Tool added and validation

`review-tools/app-inspector/webview-control.mjs snapshot` now returns rendered text, headings,
control labels/values/states/selectors, geometry, focus, scroll regions, accessibility nodes and
an optional PNG. It reuses the existing exact-executable/PID/loopback endpoint ownership checks.
The fixed reader lives in `src/rendered-state.mjs`; no caller-supplied JavaScript is accepted.
Graph buttons and tabs are supported by the existing click operation. A bounded `select`
operation supports observed single-select option values using WebView keyboard events.

```powershell
node review-tools/app-inspector/webview-control.mjs snapshot --exe 'C:/Users/user/.codex/worktrees/workflow-continuation-files/src-tauri/target/test-fast/codex-orchestrator.exe' --pid 45924 --debug-url http://127.0.0.1:9231 --target-url http://127.0.0.1:1490/ --out 'C:/Users/user/.codex/tmp/otp-usability-20260908-01/current.json' --screenshot 'C:/Users/user/.codex/tmp/otp-usability-20260908-01/current.png'
```

- 12 focused Node tests passed: rendered controls/redaction/modal context, read-only protocol
  capture, failure reporting, selected-control input and existing ownership/selector checks.
- Live snapshots succeeded against PID 45924. Passing the original demo PID with the review
  instance's port was rejected before capture and produced no receipt.
- A repeated final snapshot preserved the rendered state, focus and scroll values exactly.
- Read-only SQLite assertions passed for saved actions, node MCP restrictions, pinned recipe
  revision and the recorded Stop no-op. See `durable-verification.json` and `read-durable.py`.
- Changed JavaScript passed syntax checks and Prettier. `git diff --check` passed.

The tool requires an instance deliberately launched with a WebView debugging port. It is a
development CLI, not a new production MCP or Workflow Engine API. Snapshots exclude native
window chrome/dialogs; DOM, accessibility and image reads are sequential, not atomic. Password
values are redacted, while other visible content is included.

This pass does not establish full keyboard/screen-reader accessibility, narrower viewport
behavior, missing-package handling, app-restart persistence or real-provider cancellation.
The prior implementation's runtime tests are recorded separately in
`otp-element-selection-validation.md`; they were not rerun for this UI audit.
