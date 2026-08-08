# Operation trace: send an Agent Session message

## Product action

The user submits text through `AgentSessionComposer`. `useAgentSessionController` waits for the update subscription, captures the current Session if any, and calls the injected `AgentSessionClient.sendMessage`.

## Frontend boundary

1. `src/features/agentSessions/AgentSessionComposer.tsx`
   - Owns the form and delegates the submitted text.
2. `src/features/agentSessions/useAgentSessionController.ts`
   - Coordinates selection, subscription readiness, optimistic control state, acknowledgement identities, and reload behavior.
3. `src/application/agentSessions/contracts.ts`
   - Defines the browser-safe command and result contract.
4. `src/infrastructure/agentSessions/tauriAgentSessionClient.ts`
   - Establishes the `agent-session-update` listener before invoking `send_agent_session_message` because a fast child may emit before command acknowledgement.

## Tauri boundary

`src-tauri/src/agent_sessions/transport/mod.rs::send_agent_session_message` maps the DTO into the application command and delegates to `AgentSessionApplication::send_message`.

The same transport module converts application notifications into the `agent-session-update` Tauri event. It does not own lifecycle semantics.

## Application lifecycle

`src-tauri/src/agent_sessions/application/lifecycle.rs` performs the durable operation:

1. Ordinary `send_message` uses `AgentInvocationInputProvenance::User` and no `RuntimeLaunchExtension`.
2. A missing Session is created; an existing Session is loaded and checked for availability.
3. A pending invocation is persisted before runtime preflight.
4. Runtime mode is `Start` unless the Session already has an external context, in which case it is `Resume`.
5. Requested runtime options are preflighted into effective options.
6. The invocation is durably marked running.
7. A `RuntimeInvocationRequest` is constructed from submitted text, working directory, effective options, and any explicitly supplied launch extension.
8. The runtime starts or resumes the Codex invocation through a persisted update sink.
9. Launch acceptance is persisted separately from invocation persistence and running state.
10. Runtime updates are serialized into the repository and emitted through the notifier.

Preflight failure, launch failure, cancellation, terminal delivery, startup reconciliation, and shutdown are distinct paths.

## Runtime and process boundary

- `src-tauri/src/runtime/codex/arguments.rs` constructs `codex exec` or `codex exec resume` arguments.
- `src-tauri/src/runtime/codex/runtime.rs` owns Codex-specific launch and cancellation behavior.
- `src-tauri/src/runtime/codex/protocol.rs` normalizes JSONL runtime output.
- `src-tauri/src/runtime/processes/` supervises operating-system child processes.

Role-specific product services can opt into `RuntimeLaunchExtension`, which adds configuration, environment, or an application-provenance prompt prefix. Generic user sends cannot acquire this extension accidentally.

## Return path

Persisted update notifications become `agent-session-update` events. The frontend bridge fans them out to subscribers, and the controller reloads the authoritative Session history for rendering through the transcript projection and viewport.

## Architectural reading

This is one of the cleanest vertical slices in the product. DTO transport, lifecycle semantics, persistence, provider adaptation, and process supervision have recognizable boundaries. The important coupling seam is `RuntimeLaunchExtension`: it is deliberately neutral but enables orchestration, Harness, MCP, and native-profile policy to affect a particular Codex child.
