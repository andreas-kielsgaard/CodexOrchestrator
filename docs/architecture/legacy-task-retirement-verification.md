# Legacy task retirement: real agent verification

Verified on 2026-09-14 against cleanup commit `ac18781` on
`cleanup/quarantined-legacy-task-code` in
`C:\Users\user\.codex\worktrees\legacy-task-retirement`.

The real agent checks passed, with one performance caveat: cancellation took about 20 seconds
from the UI click receipt to the application's persisted canceled state. The cause of that delay
was not established. The relevant cancellation implementation is unchanged by this cleanup.

## Real agent interaction

Ran the built desktop app with disposable application data, a separate Codex home, and a small
test workspace. Used authenticated Codex app-server with `gpt-5.6-sol` in read-only mode. These
were real provider invocations; the earlier desktop smoke check used seeded history.

| Check                              | Observed result                                                                                                                             |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Send a message and use a file tool | Agent executed `Get-Content` on the test marker file, received exit code 0, and returned its exact contents. The reply appeared in the app. |
| Follow up through the UI composer  | Agent returned the earlier marker without using a tool. The UI showed the completed reply.                                                  |
| Cancel through the UI button       | Provider reported `interrupted`, the app persisted `canceled`, and the managed process exited with code 0.                                  |
| Continue after cancellation        | The same conversation returned the original marker again without tools. The saved reply appeared after refreshing the app's history.        |

All five invocations used the same native provider thread. The additional invocation was an
initial cancellation scenario using `Start-Sleep`; policy rejected that command, so it completed
without a wait. Cancellation was then checked using a long text response, without changing policy.

The initial message and recovery message used the app's native commands through its owned
WebView. The follow-up and cancellation scenarios used the actual composer and buttons. Recovery
submitted through the native command did not automatically enter the open transcript; clicking
Refresh loaded it. The existing controller ignores updates for invocation IDs it has not loaded.

Cancellation timing was `16:39:58.830Z` at the click receipt and `16:40:18.429Z` at the persisted
canceled state: 19.599 seconds. Its raw provider terminal was `interrupted`; existing normalization
records that as provider `failed`, while the application correctly records `canceled`. A full UI
snapshot timed out on this event-heavy turn; focused DOM reads succeeded and confirmed the final
status and usable composer. Treat the delay as a separate investigation, not a proven cleanup
regression.

These checks cover basic conversation, file-tool use, cancellation, and continuation. They do not
establish every provider feature, approval interaction, or interruption of a running shell tool.
No further runtime checks of unrelated product areas were performed in this follow-up, as requested.

## Code analysis of the remaining impact

Compared `e2bfc6c` with `ac18781`, including the separate disconnected-UI cleanup in `67b325a`.

- Parsed frontend imports, re-exports, type imports, and literal dynamic imports. None of the 109
  deleted frontend files were reachable from `src/main.tsx` before deletion. Of these, 101 belong
  to this retirement; eight belong to the separate cleanup. No nonliteral dynamic imports were
  found in either revision.
- Compared selectors, surrounding at-rules, declaration values, and order. All 64 retained global
  CSS rules and 61 retained Workflow authoring rules remain identical. Manual review of the two
  ambiguous removed class names, `workspace` and `count`, found only non-class string uses in
  retained source. The removed Workflow selection variant `run` had no retained producer or reader.
- Current Agent Session UI, application cancellation lifecycle, Tauri Session commands, and Codex
  app-server implementation are unchanged. Current startup composition remains; only the ten
  retired task commands were removed from its registrations.
- Current storage, persistence, product database, Workflow runtime, repository services, and
  Worktree Review services are unchanged. The removed prototype-table quarantine helper was
  called only by the deleted legacy migration registry. Active-v3 schema and migration helpers,
  including old-file preservation and current Session reopen coverage, remain intact.

No production code changed during this verification. The prior build/test results remain recorded
in the [retirement plan](legacy-task-retirement-plan.md#execution-record).

## Evidence and teardown

- Session: `f3f0702e-3c31-496f-a2a3-f7730461ed8a`.
- Provider thread: `01a0a0c2-e246-7272-9480-642fc3be0fd7`.
- Tested executable SHA-256:
  `cde73c7576ac7a1718f48c603e6763c1f24cdcb6d650a3b7164884cac42bd420`.
- Local command receipts, provider events, UI captures, code-analysis output, and aggregate
  assertions: `.dev/legacy-task-retirement/real-agent-check/` (Git-ignored).

The owned test app was closed gracefully and its temporary credential copy was removed.
The user's original credential file and unrelated app instances were left in place.
