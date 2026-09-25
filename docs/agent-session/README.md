# Agent Sessions

An Agent Session is a durable interaction context: a conversation, its working directory, its configuration references, and the provider context needed to continue the work. Its visible transcript is one view of that context.

This guide describes the integrated Agent Sessions implementation (14 September 2026). [Execution configuration](../execution-configuration.md) explains profiles and native settings; [Workflows](../workflows.md) explains application-directed delivery between sessions.

## Start and continue a session

1. Select a native Codex home in **Technical Settings**, then save and choose a default Capability Profile in **Capabilities**.
2. Open **Agent Sessions** and choose **New session**. Supply an existing absolute working directory if the work belongs in one.
3. Submit a message. The application validates the first message's configuration, creates the session, persists the invocation, and launches Codex.
4. While it works, inspect processing activity, answer a supported provider request, steer the active turn, or cancel it.
5. Reopen the session later and send another message to continue its retained provider context.

A blank working-directory choice allocates an initially empty directory beneath `~/.codex-orchestrator/workspaces/<session-id>`. The directory and its ownership marker are retained across turns and app restarts. This gives an ordinary conversation an intentional working context instead of accidentally using the app's checkout. An explicitly selected directory remains the caller's directory; allocation does not copy a repository into it.

Type `/` in the composer to discover model, reasoning and skill choices from the selected runtime. Model and reasoning choices apply to the next human message; skills insert their provider-owned invocation text. These choices do not edit the Session Profile or later Workflow defaults. A default Capability Profile is required for a new ordinary session; an older session without a pinned profile remains readable but cannot use the current profiled send route. See [birth and continuation](../execution-configuration.md#birth-and-continuation) for the exact lifetimes.

Session settings show the assigned identity and offer **Set identity** or **Edit identity** independently of the pinned execution policy. **Session Event deliveries** shows recorded Workflow deliveries, exposes read errors and provides **Refresh deliveries**. The shared pane wires both features for existing sessions.

The sidebar groups registered repositories into reorderable Sessions and Workflows trays. Workflow instances separate added conversations from workflow-owned sessions. Placement and pins are visual organization: moving a session never changes its working directory or workflow ownership. Creating a session within a repository or workflow folder uses the repository main working tree, including composer discovery before the first message.

Groups show five sessions initially with **Show more**; Pinned shows all. Drag to reorder folders, conversations and pins. Hover actions pin or unpin, create sessions, or open the owning workflow and node. Right-click a session to copy its deeplink. Opening a pinned shortcut preserves collapsed folders and highlights its ancestor. Agents can use the [local UI command interface](navigation-commands.md).

## Identity and lifecycle

| Identity or state    | Meaning                                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------------------------- |
| Local Session ID     | Stable application identity used to load the conversation and relate it to product records.              |
| Invocation ID        | One submitted turn and its lifecycle. A session can contain many invocations.                            |
| Provider context ID  | The selected provider's external context identity, scoped to that provider/configuration/device.         |
| Session availability | Whether the session can accept work; completing a turn does not close the session.                       |
| Invocation status    | Pending, running, completed, failed, canceled, or interrupted.                                           |
| Runtime interaction  | A steering input or provider request correlated to the active invocation, with its own recorded outcome. |

Only one invocation can be active in a session. Separate sessions can run concurrently. A follow-up resumes the external provider context when one exists; neither the local Session ID nor the invocation ID is a substitute for it.

Production currently registers **Codex app-server** behind the provider-neutral `AgentRuntime` port. Provider runtime, configuration, options, interaction encoding, and continuation are separate contracts; see the [agent provider boundary](../architecture/agent-provider-integration.md). The Codex adapter owns executable resolution, native messages, start/resume, steering, requests, and interruption.

Once a session has a concrete provider/device/configuration/capability-profile/workspace binding, selecting a different binding creates a destination session. The source session and its history remain unchanged. Provider-native continuation can be installed only through the matching provider's continuation implementation.

Each invocation has a supervised app-server process. The supervisor owns process handles, input/output, terminal observation, and shutdown. On Windows, the factories launch suspended children and attach a kill-on-close Job Object before resuming them. The old July recovery evidence's direct-child-only limit belongs to that older checkpoint.

Cancel requests interruption of the active invocation. Closing or navigating away from a view is a different action. On startup, reconciliation applies recoverable terminal evidence first, then marks remaining active records interrupted because their in-process owner did not survive. It does not silently resend the prompt or claim to reattach an unknown process.

## Durable conversation and live interaction

The application stores submitted text, ordered invocations, normalized runtime events, raw provider payloads, and diagnostics. Runtime updates are persisted before UI notifications. The frontend subscribes before sending and can reload durable history when notifications are missed. Notifications make the view responsive; they are not the sole record of completion.

The transcript intentionally shows processing and tool activity while a turn runs. After completion, processing collapses into an expandable disclosure and the final response remains prominent. Reopening reconstructs all invocations and their events, rather than only the latest provider log. Technical payloads stay available through secondary inspection.

Steering addresses the current provider turn. The application records an input identity, pending state, and accepted/rejected/uncertain result. Repeating the same input identity and content returns the recorded result; reusing that identity for different content is rejected. Accepted steering is a distinct Session notification and does not trigger Workflow connection delivery.

Supported provider approvals, permission requests, and questions appear with their supplied choices or input fields. Responses are tied to the still-pending request and active invocation. Invalid responses can leave a request pending; uncertain transport outcomes remain uncertain. Unsupported requests are displayed as unsupported. This describes the implemented interaction surface, not complete parity with every Codex client feature.

Runtime outcome, persistence failure, notification failure, and downstream product processing are separate facts. A failed Workflow handoff must not turn the sender's completed invocation into a provider failure. Durable diagnostics retain failures at their actual boundary.

## Product-owned sessions and shared presentation

Ordinary sessions need no Epic, Workflow, or task association. Product services can create sessions with explicit ownership, an initial prompt contribution, or an application-owned invocation identity. The service that owns the product relationship also owns its authorization and transition rules.

Workflow node conversations reuse `ProfiledSessionPane` and `AgentSessionWorkspace`. Epic/Plan Builder and other retained product hosts also consume the shared conversation behavior. A host supplies context and navigation; it does not need a second transcript, composer, cancellation implementation, or provider adapter. Session functionality is independently composed; the retired Task stack is absent from current startup.

Session history is not an orchestration event store. A model's prose, final response, or visible skill reference does not by itself accept a plan, settle a Work Unit, or mutate structured product state. Those effects require the owning application's semantic command. [Retained orchestration](../orchestration/README.md) describes that separate system.

## Implementation ownership

| Responsibility                                             | Current source                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Domain, runtime and repository contracts                   | [`agent_sessions/domain.rs`](../../src-tauri/src/agent_sessions/domain.rs), [`ports/`](../../src-tauri/src/agent_sessions/ports/)                                                                                |
| Creation, invocation, updates, requests and reconciliation | [`agent_sessions/application/`](../../src-tauri/src/agent_sessions/application/)                                                                                                                                 |
| Ordered history, profiles and logical addresses            | [`agent_sessions/repository/`](../../src-tauri/src/agent_sessions/repository/)                                                                                                                                   |
| Retained workspace allocation                              | [`agent_sessions/workspace.rs`](../../src-tauri/src/agent_sessions/workspace.rs)                                                                                                                                 |
| Codex protocol and process ownership                       | [`runtime/providers/codex/app_server/`](../../src-tauri/src/runtime/providers/codex/app_server/), [`runtime/processes/`](../../src-tauri/src/runtime/processes/)                                                   |
| Production selection and notification fan-out              | [`active_app/sessions.rs`](../../src-tauri/src/active_app/sessions.rs), [`session_notifications.rs`](../../src-tauri/src/active_app/session_notifications.rs)                                                    |
| Frontend clients, collection, conversation and projection  | [`application/agentSessions/`](../../src/application/agentSessions/), [`infrastructure/agentSessions/`](../../src/infrastructure/agentSessions/), [`features/agentSessions/`](../../src/features/agentSessions/) |

`useAgentSessionCollection.ts` owns collection loading and mutations; `useAgentSession.ts` owns one conversation. `useProfiledAgentSession.ts` composes configuration and delivery for standalone and workflow panes. `session_navigation/` composes repository facts and organization without changing session execution ownership. The [architecture guide](../architecture.md) and [ActiveDatabase reference](../architecture/active-database.md) own composition and transaction rules.

## Why these boundaries exist

The recovery review found that local IDs were being used as provider continuation IDs, frontend memory was being treated as durable storage, only the newest log was reloaded, and objects named as process owners had no process handle. The rebuilt model separated identity, invocation, persistence, provider access, and presentation so each could be tested at its real boundary. Final-first display was an explicit user choice.

Later work moved production to app-server for interactive steering and requests, retained empty workspaces for ordinary sessions, and integrated that bounded Session implementation into main without importing OTP. Historical test counts, native/provider fixture results, and residuals belong in [validation evidence](../validation-evidence.md); current source inspection is not a new live validation run.

Original references:

- **“Review agent session view merge”**, task `019f48bb-85b0-7451-bf2c-5483a36a18ff`: original rollout `rollout-2026-07-09T23-14-44-019f48bb-85b0-7451-bf2c-5483a36a18ff.jsonl`, user messages at lines 424, 645 and 823 establish the interaction-context model, recovery boundary, and presentation intent. The historical failure record is `e2bfc6c:docs/agent-session/evidence.md`.
- **“Codex Feature Parity”**, task `01a0815c-71b3-7423-857e-07009d705763`: original rollout `rollout-2026-09-08T16-11-54-01a0815c-71b3-7423-857e-07009d705763.jsonl`, user messages at lines 109 and 316 establish workspace/default/interaction direction. Turn `01a08589-3ce2-7f51-990c-7491a340874f` records integration at `e914a9e`; turn `01a0858d-2d71-7a80-bcf5-db7d3da60fc8` records main publication alongside `ac22f01`. The bounded implementation evidence is `e2bfc6c:docs/agent-session/session-main-integration.md`.

## Frontend checks and recorded previews

The tooling cleanup retired the standalone recorded Session page and its simulator. `AgentSessionScreen.test.tsx` checks update routing, durable history reload and collection errors with explicit client responses; transcript, Markdown, controller and profile suites retain their focused coverage. Run `npm test -- src/features/agentSessions` for the focused frontend suite. Other [developer previews](../development.md#recorded-review-routes) retain fixed histories with unsupported Session mutations. See the [cleanup validation record](../cleanup/tooling-build-cleanup-validation.md) for executed checks.

## Separate development work

The [repository navigation plan](repository-session-navigation-plan.md) records the design decisions implemented by the navigation feature.

The separate remote implementation at `ab220ff` targets ordinary Codex sessions on existing local/remote worktrees. Its plan and setup record are available as `ab220ff:docs/agent-session/remote-worktree-session-plan.md` and `ab220ff:docs/agent-session/remote-worktree-server-setup.md`. The original checkout's untracked plans remain with their owning task. That remote target picker remains separate; see [work outside this checkpoint](../README.md#work-outside-this-checkpoint).
