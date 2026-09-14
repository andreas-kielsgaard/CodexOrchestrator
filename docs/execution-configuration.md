# Execution configuration

Execution configuration connects a session to a native Codex environment and records the product choices used to run it. The important distinction is between **what is configured**, **what is currently available**, and **what a particular invocation actually received**.

This guide describes main `60c3798` (14 September 2026). [Agent Sessions](agent-session/README.md) owns conversation and lifecycle behavior; [Workflows](workflows.md) owns recipe-driven creation and delivery.

## Configuration layers

| Layer                     | What it owns                                                                                                                           | Lifetime                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Native Codex home         | Filesystem identity, native configuration, authentication and native skill/tool environment selected through `CODEX_HOME`              | Registered independently; continuity checked before use.                                        |
| Runtime Profile           | Observed models, reasoning modes, sandbox capabilities, skills and application-provided tools for the selected runtime                 | Read from the selected home and working context.                                                |
| Capability Profile        | A saved set of allowed capabilities and optional model/reasoning/sandbox defaults                                                      | Revisioned shared configuration; one saved profile is selected as the ordinary-session default. |
| Node Profile              | A Workflow node's embedded allowed capabilities and pinned defaults                                                                    | Part of its recipe; copying produces independent values.                                        |
| Session Profile           | The resolved creation result: runtime reference, observed exposure, allowed managed capabilities, defaults and source profile revision | Stored with the session with an integrity digest.                                               |
| Agent Identity            | Assigned name/initials, color and shape used to present the session                                                                    | Presentation data; it does not confer capability or product authority.                          |
| Conversation Harness      | Retained product-owned guidance and runtime/tool configuration for a particular orchestration role                                     | Resolved by its owning product service/catalog/revision.                                        |
| Technical Harness binding | The session's allowed product MCP exposure and its invocation correlation                                                              | Durable logical policy, with current process connections prepared at launch.                    |

A native home is not a named Codex `--profile`, a Capability Profile, an Agent Identity, or a repository. Selecting one does not imply that sessions in other products share conversations. A presentation identity can change without changing runtime policy.

These layers were separated so prompt guidance, identity, capabilities, and invocation choices could evolve without one mixed “Harness” object becoming the authority for everything. New Workflow configuration uses Capability/Node/Session Profiles; retained Epic/Sprint services still have their own Conversation Harness contracts.

## Native home and readiness

**Technical Settings** can discover or register an existing native home, register a manual path, create a dedicated home, and select the home used by the application. The service records filesystem continuity as well as a path. Replacing or moving the underlying home is not automatically the same identity.

The settings surface also records browser-login attempts, authentication status, sandbox setup or external adoption, execution-mode authorization, canaries, and MCP/reporting checks. Each answers a different question:

- A login request or browser handoff does not establish authenticated state.
- A setup process finishing, UAC confirmation, external configuration observation, and adoption confirmation are separate records.
- A canary request, accepted process launch, provider activity, expected receipt, cleanup, and readiness are separate observations.
- The retained Danger Full Access mode has an explicit full-machine/unrestricted-network authorization record; choosing the mode alone does not create that authorization.

These administrative records are not a universal checklist that every ordinary Session must complete. The current Session launch binding requires the selected home to be active and retain its registered filesystem identity; it returns readiness separately. Provider configuration/discovery and the actual launch can still fail for their own reasons.

The application is the sole source of `CODEX_HOME` for this path. It rejects a caller-supplied override, stores the session-to-home binding, and checks it again on later launches. Selecting a different home does not retarget an already-bound session: a mismatch produces an error rather than silently continuing under another native identity.

## Birth and continuation

A new ordinary session uses the explicitly selected default Capability Profile. The application constructs its creation input without inventing a hidden Workflow node, validates the first human message's choices before session persistence, and stores the resolved Session Profile. A first-message override does not become a pinned default.

A Workflow instance stores its recipe snapshot when the instance is created. Each node session resolves the referenced Capability Profile and runtime when that session is born. This means two sessions born at different times can resolve different revisions of a shared Capability Profile even within one instance. Existing addressed sessions continue from their stored result; deleting a shared profile can block a new birth but does not require re-reading that catalog entry to address an existing session.

At birth, allowed managed exposure narrows through the layers:

```text
observed runtime capabilities
  -> saved Capability Profile
  -> embedded Node Profile
  -> resolved Session Profile
```

The resolver rejects widening, excluded runtime locks, unavailable defaults, and conflicting locked selections. Profile defaults are overlaid with node defaults; explicit node values take precedence. The Session Profile records the source revision and resolved values, protected by a digest. Its inspector is read-only.

**Pinned does not mean every native setting is frozen forever.** An explicit pinned value remains a product choice. An absent value means inheritance: the adapter lets Codex resolve native settings for that invocation. On continuation, app-server resolves current native model/reasoning/approval/sandbox defaults for the same home and working context before resuming the existing provider thread. The resulting effective configuration is recorded in runtime events. This avoids accidentally treating defaults retained in provider history as the current native defaults.

The selected runtime reference must still match the stored Session Profile. Direct-message choices are checked against current runtime exposure. The code does not silently replace an unavailable or mismatched runtime.

## Direct and managed invocation choices

| Input                                                 | Resolution                                                                                                             |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Human message with an explicit model/reasoning choice | Uses that choice for this invocation, within the full attached runtime exposure and any runtime lock.                  |
| Human message without an explicit choice              | Uses an explicit pinned profile default when present; otherwise inherits native behavior.                              |
| Managed Workflow message                              | Uses the target session's resolved managed policy/defaults. It does not inherit a previous human message's override.   |
| Explicit sandbox request                              | The adapter checks the effective sandbox returned by Codex before starting the turn; a mismatch fails visibly.         |
| Inherited sandbox/approval policy                     | Native configuration supplies the policy; the Session binding's old readiness/mode records are not substituted for it. |

The direct-user composer exposes model and reasoning. The underlying direct invocation contract also represents sandbox selection. It is misleading to describe every backend selection as a mounted UI control.

For example, a node may pin a particular model for Workflow messages. A human can choose another model exposed by the same native runtime for one message; the next managed message still uses the node session's profile policy. Changing the shared Capability Profile likewise does not rewrite that session's stored result.

The native environment reader uses the same Codex executable, app-server protocol and process owner as execution. It reads configuration, requirements, model and skill information for the working context. Capability discovery failure is an unavailable observation, not proof that a capability is unsupported. The old test-only CLI-help adapter's cache lifetimes are not the current discovery contract.

## Harness configuration and delivery

Two retained mechanisms have different jobs.

**Product Conversation Harnesses** provide versioned guidance and execution intent for Epic/Sprint/Work Unit conversations. The owning product service supplies role-specific tools, runtime constraints and, where appropriate, a first-query application-provenance prefix. Generic Session code receives a neutral prefix value and preserves the user's submitted text unchanged. The initial prefix is not reinjected merely because the app restarted. A declared skill name/path describes guidance; it is not evidence that a model read or applied that skill.

The retained Harness catalog resolves exact version references through explicit replacement records. Session-owned references can be updated to the resolved version before an invocation. That is distinct from rewriting an already-delivered prompt or mutating a pinned Session Profile. Product-specific working copies and revision acceptance remain owned by [orchestration](orchestration/README.md), not by the ordinary Session composer.

**Technical Harness bindings** mediate application-provided MCP tools. A pinned Session Profile can create this binding directly, without a Workflow Role or mixed Harness catalog entry. The durable exposure policy stores logical server/tool selections; launch preparation resolves current upstream connections, registers the binding, establishes invocation correlation, and supplies proxy connections to Codex. Process URLs and tokens are not frozen as durable profile configuration.

Product MCP connections are added to native configuration. A name collision is rejected instead of replacing a native connection. The managed proxy filters the tools it mediates and supplies trusted invocation context. Tool visibility is not the final authorization boundary: the receiving application command still validates its own authority. Neither a capability list nor a skill declaration should be read as an independent filesystem sandbox or exhaustive removal of all inherited native tools and skills.

The adapter preserves explicit product constraints. For example, a retained Harness requiring user/project rules to be ignored cannot quietly lose that requirement when the selected app-server cannot honor it. The adapter permits that path only where ignoring those rules is demonstrably a no-op; otherwise launch fails before the turn.

## Reading inspectors correctly

Keep four facts separate when describing configuration:

| Fact               | What it can establish                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------ |
| Configured context | The current catalog, profile, or working-copy values.                                      |
| Delivery evidence  | What a particular invocation was actually supplied or what its durable record establishes. |
| Editability        | Which owner permits changes to a value and at what stage.                                  |
| Validation         | Whether a defined check passed, failed, or remains unverified.                             |

A recorded development view may display editable configuration and simulated Commit/Push controls without a live product mutation or provider-delivery path. It must not label that as immutable delivered context. Invalid configuration is also different from an unverified configuration. The [recorded review package](../offline-review/README.md) and [validation evidence](validation-evidence.md) retain the limits of those explorations.

## Implementation ownership

| Responsibility                                                   | Current source                                                                                                                                                                                                                                     |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native registration, continuity and readiness                    | [`native_profiles.rs`](../src-tauri/src/native_profiles.rs), [`native_profiles/`](../src-tauri/src/native_profiles/)                                                                                                                               |
| Native settings UI                                               | [`NativeProfileSettings.tsx`](../src/features/nativeProfiles/NativeProfileSettings.tsx)                                                                                                                                                            |
| Profiles, defaults, resolution, digest, shared catalog           | [`execution_configuration/`](../src-tauri/src/execution_configuration/)                                                                                                                                                                            |
| Codex environment observation and effective launch configuration | [`app_server/environment.rs`](../src-tauri/src/runtime/codex/app_server/environment.rs), [`app_server/mod.rs`](../src-tauri/src/runtime/codex/app_server/mod.rs), [`configuration.rs`](../src-tauri/src/runtime/codex/app_server/configuration.rs) |
| Ordinary profiled first/next message                             | [`agent_sessions/application/configuration.rs`](../src-tauri/src/agent_sessions/application/configuration.rs)                                                                                                                                      |
| Addressed profile creation and managed send                      | [`agent_sessions/session_event_adapter.rs`](../src-tauri/src/agent_sessions/session_event_adapter.rs)                                                                                                                                              |
| Product guidance, working copies and accepted revisions          | [`orchestration/conversation_harness.rs`](../src-tauri/src/orchestration/conversation_harness.rs), [`conversation_harness_revision.rs`](../src-tauri/src/orchestration/conversation_harness_revision.rs)                                           |
| Logical MCP policy, binding and current connections              | [`harness_engine/exposure.rs`](../src-tauri/src/harness_engine/exposure.rs), [`session_binding.rs`](../src-tauri/src/harness_engine/session_binding.rs), [`launch.rs`](../src-tauri/src/harness_engine/launch.rs)                                  |
| Profile editing and Session inspection                           | [`features/executionConfiguration/`](../src/features/executionConfiguration/), [`useSessionExecutionSelection.ts`](../src/features/agentSessions/useSessionExecutionSelection.ts)                                                                  |

## Decision provenance and limits

- **Profile separation:** “Workflow to Session Event Overhaul”, task `01a0390e-64bb-7b01-b277-e019f37d51bc`, original rollout `rollout-2026-08-25T15-13-59-01a0390e-64bb-7b01-b277-e019f37d51bc.jsonl`, user messages at lines 1396, 1423, 3070 and 3132. These establish separate capabilities, embedded node values, read-only Session resolution, independent identity, and direct-user freedom. The pre-consolidation contract account is `e2bfc6c:docs/session-event-model/contract-rules.md`.
- **Default profile and native inheritance:** “Codex Feature Parity”, task `01a0815c-71b3-7423-857e-07009d705763`, original rollout `rollout-2026-09-08T16-11-54-01a0815c-71b3-7423-857e-07009d705763.jsonl`, user messages at lines 109, 239 and 316; implemented main port `e914a9e`. This supersedes the synthetic ordinary-session default proposed in `e2bfc6c:docs/session-event-model/repair-plan/README.md`, D1.
- **Configured versus delivered:** “Review agent session view merge”, task `019f48bb-85b0-7451-bf2c-5483a36a18ff`, original rollout `rollout-2026-07-09T23-14-44-019f48bb-85b0-7451-bf2c-5483a36a18ff.jsonl`, review messages at lines 10425 and 10594 record the truthfulness correction and bounded independent acceptance of `f3e332a`. The original four-way distinction is `e2bfc6c:offline-review/explorations/harness-inspector/ARCHITECTURE.md`.

Current source establishes the configuration behavior above. Historical native checks, local provider fixtures, canary results, and missing original NCHP production history are qualified in [validation evidence](validation-evidence.md). The remote-development proposal to bind each Capability Profile to a device belongs to `ab220ff:docs/agent-session/remote-worktree-session-plan.md`, not main's selected-local-home model; see [work outside this checkpoint](README.md#work-outside-this-checkpoint).
