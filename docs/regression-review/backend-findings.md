# Backend regression review

Reviewed `4bded63` against `9fc822f` on 2026-09-07. This is a source review of the mounted application paths. No provider was started, no live messages were sent, and no product code was changed. The checks below are code-path evidence, not live-provider tests.

Role removal, embedded Node Profiles, pinned Session Profiles, and per-message model/reasoning choices are accepted changes. The temporary model/reasoning lists and deferred provider controls are not treated as regressions.

## B1 — Ordinary sessions cannot receive their second message (P1)

The first message in a new standalone session uses the generic send command. That command creates a session with no Session Profile. Once the new session is selected, the screen routes all later messages through the new profile client. That route requires a pinned profile and rejects the session.

Existing standalone sessions and sessions made by the older Workflow path have the same problem. The underlying generic send command still works; the mounted screen no longer uses it for an existing session.

Evidence:

- `src-tauri/src/agent_sessions/application/lifecycle.rs:679`: first send creates an ordinary session.
- `src-tauri/src/agent_sessions/application/lifecycle.rs:248-252`: ordinary creation uses default ownership, with no profile.
- `src-tauri/src/agent_sessions/transport/dto.rs:35-40`: explicit ordinary creation also sets `session_profile: None`.
- `src/features/agentSessions/AgentSessionScreen.tsx:130-149`: all selected sessions use the profile route; missing profile throws `Pinned Session Profile is not available for this Session.`
- `src-tauri/src/agent_sessions/application/session_profile.rs:95-105`: backend profile lookup rejects the same missing profile.

Safe reproduction: seed or create an ordinary session without invoking a provider. Open it in Agent Sessions and attempt a send with a fake client/runtime. Profile lookup fails before any second invocation can launch. A complete first-message/second-message test should use the real creation path and a fake runtime.

The [saved browser probe](verification.md#browser-reproduction) also reproduced the UI half of this path: one generic first send, then a missing-profile error on the second send, with neither a second generic send nor a profile send. Its creation response is fake; the source trace above establishes the real no-profile creation behavior.

## B2 — New workflow connections never run from their triggers (P1)

The new screen can save and compile connections, but the running application only dispatches the starting user event. Completion of that session does not dispatch the next connection. MCP calls, application events, and event-group completion also have no production route into the new workflow execution service.

Evidence:

- `src-tauri/src/workflows/execution.rs:25-60`: user-entry dispatch is implemented.
- `src-tauri/src/workflows/execution.rs:64`: `dispatch_compiled_occurrence` has no caller anywhere in `src-tauri/src`.
- `src-tauri/src/active_app.rs:45`: the runtime completion callback still calls the old `WorkflowApplication`.
- `src-tauri/src/active_app.rs:294-306`: MCP and workflow notifications are wired to that old application too.
- `src-tauri/src/workflows/compiler.rs:117-148`: trigger definitions are produced, but compilation does not install a trigger listener.

Safe reproduction: activate an A → B recipe whose connection uses “Invocation completed.” Dispatch A through a fake runtime and emit its terminal notification. The new path has no handler that selects and dispatches the B definition. Existing compiler tests prove definition construction; the execution test uses a fake dispatcher and stops at the first request.

## B3 — New workflow sessions lose their chosen worktree (P1)

The new send contract takes only recipe ID, instance ID, and text. There is no target path to resolve. New addressed sessions set their working directory to `None`, so the child process inherits the application's current directory. That can be unrelated to the repository the user wants the workflow to work in.

Evidence:

- `src-tauri/src/workflows/execution_transport.rs:21-25`: the input has no repository/worktree target.
- `src-tauri/src/agent_sessions/session_event_adapter.rs:244-254`: session creation explicitly supplies `working_directory: None`.
- `src-tauri/src/runtime/processes/system.rs:20-21`: the process directory is only set when one was supplied.
- The prior path still supplies the instance worktree in `src-tauri/src/workflows/node_sessions.rs:169` and `:229`.

Safe reproduction: use a fake runtime to capture the first new workflow invocation and inspect its working directory. It is `None`. No provider run is needed to see the missing target.

This is a functional consequence of the missing instance creation path, not just a missing form field.

## B4 — Editing shared definitions can break an already pinned session (P1)

Each workflow send reloads the current active recipe and every current Capability Profile before it looks for an existing session. Narrowing or deleting a Capability Profile can therefore block a later workflow message to a session which already has its own valid pinned profile. Sending directly to that same session can still work.

Evidence:

- `src-tauri/src/workflows/execution.rs:34-36` and `:71-73`: both workflow send paths compile again.
- `src-tauri/src/workflows/authoring_service.rs:113-148`: compilation uses the current active recipe and reads current profiles by ID.
- `src-tauri/src/workflows/authoring.rs:291-311`: current profile narrowing can reject a node before session lookup occurs.
- `src-tauri/src/execution_configuration/service.rs:107-121`: a Capability Profile can be deleted without regard to existing recipe references.

Safe reproduction: with fake runtime ports, create a session from a recipe; then narrow or delete its Capability Profile; send again using the same recipe and instance ID. Compilation now fails before the pinned session can be reused.

There is also no stored new workflow instance that pins a recipe revision. Activating a draft replaces `active_json` (`authoring_repository.rs:142-147`), and later sends for the same instance string use that replacement. The old instance path stored a recipe ID and target (`workflows/repository/instances.rs:105-126`). Restoring only the instance list UI would not restore these guarantees.

## B5 — Activation accepts trigger/source pairs that cannot deliver (P2)

A connection can be activated with an MCP-call trigger and an “Invocation output” prompt source. That source can never be read from an MCP occurrence. The same issue applies to other trigger-specific sources paired with the wrong trigger.

Evidence:

- `src-tauri/src/workflows/compiler.rs:150-175`: prompt inputs are copied without checking the trigger.
- `src-tauri/src/session_events/domain.rs:352-373`: definition validation checks shapes and required fields, but not this pairing.
- `src-tauri/src/session_events/materialization.rs:160-168`: reading invocation output from any other trigger always returns a missing-value error.

Safe reproduction: build a valid two-node recipe, choose `McpCall`, leave `InvocationOutput` as the prompt input, and activate. It compiles. Materializing a matching MCP occurrence then fails. This can be verified with the compiler and materializer alone.

## Boundaries that still need a clear status

These are not counted as additional confirmed regressions:

- **MCP and skills:** the new adapter creates no Harness Engine binding (`session_event_adapter.rs:252`). An unbound launch skips that engine (`harness_engine/service.rs:326-328`). The new app currently advertises empty MCP/skill sets and fixture model/reasoning values (`active_app.rs:185-207`). This matches part of the accepted mock scope, but the new profile is not an MCP enforcement contract yet. Do not describe the new path as preserving the old managed MCP handoff behavior.
- **Direct user event history:** `AgentSessionProfileApplication` sends directly to Agent Sessions (`application/session_profile.rs:131`), so these messages do not produce Session Event delivery rows. The transcript remains available. Whether to record the implicit UI user event is a follow-up choice; it is not required to fix B1.
- **Fixed prompt references:** node and connection text compile to `Literal` (`workflows/compiler.rs:176-186`). The text is preserved, but its individual node/connection field reference and revision are not. Source links promised for a later message inspector will need more than the current literal record.

## Suggested repair checks

These are small, focused checks for the next implementation pass:

1. New standalone session: first send, then second send, with a fake runtime.
2. New workflow: create a real instance with a target; capture the first invocation directory.
3. Complete node A through the normal notifier; observe one delivery to node B.
4. Change a shared profile after session creation; address the existing pinned session again.
5. Reject a prompt source that the chosen trigger cannot provide before activation.

No retry machinery, new script language, provider discovery work, or run graph is needed to prove these happy paths.
